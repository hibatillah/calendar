use super::*;

impl GoogleAuth {
    pub async fn begin(&self) -> Result<String, String> {
        let config = self.config.as_ref().ok_or(
            "Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET in .env, then restart the backend.",
        )?;
        let mut state = self.state.lock().await;
        self.initialize(&mut state).await;
        if state.pending.is_some() {
            return Err("Sign-in is already open. Finish it or cancel before retrying.".into());
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| "Unable to start local sign-in callback.")?;
        let redirect_uri = format!(
            "http://127.0.0.1:{}/",
            listener
                .local_addr()
                .map_err(|_| "Cannot read callback port.")?
                .port()
        );
        let verifier = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let nonce = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let mut url = reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
        url.query_pairs_mut().extend_pairs([
            ("client_id", config.client_id.as_str()),
            ("redirect_uri", &redirect_uri),
            ("response_type", "code"),
            ("scope", SCOPE),
            ("state", &nonce),
            ("code_challenge", &challenge(&verifier)),
            ("code_challenge_method", "S256"),
            ("access_type", "offline"),
            ("prompt", "consent select_account"),
        ]);
        state.pending = Some(Pending {
            state: nonce.clone(),
            verifier,
            redirect_uri,
        });
        state.message = None;
        let auth = self.clone();
        tokio::spawn(async move {
            let callback_auth = auth.clone();
            let callback_nonce = nonce.clone();
            let (done, completed) = tokio::sync::watch::channel(false);
            let router = Router::new().route("/", get(move |Query(parameters): Query<HashMap<String, String>>| {
                let auth = callback_auth.clone(); let done = done.clone(); let expected_state = callback_nonce.clone();
                async move {
                    let result = if parameters.get("state") == Some(&expected_state) { auth.complete(parameters).await } else { Err(()) };
                    if result.is_ok() { let _ = done.send(true); }
                    ([("cache-control", "no-store"), ("referrer-policy", "no-referrer"), ("content-security-policy", "default-src 'none'")], Html(if result == Ok(true) { "<h1>Google connected</h1><p>Return to Calendar. You can close this tab.</p>" } else { "<h1>Sign-in did not complete</h1><p>Return to Calendar for details. You can close this tab.</p>" }))
                }
            }));
            let shutdown = async move {
                let mut completed = completed;
                let _ = completed.changed().await;
            };
            let _ = tokio::time::timeout(
                Duration::from_secs(300),
                axum::serve(listener, router)
                    .with_graceful_shutdown(shutdown)
                    .into_future(),
            )
            .await;
            let mut state = auth.state.lock().await;
            if state
                .pending
                .as_ref()
                .is_some_and(|pending| pending.state == nonce)
            {
                state.pending = None;
                state.message = Some("Google sign-in timed out. Try connecting again.".into());
            }
        });
        tracing::info!(action = "google_sign_in_started");
        Ok(url.into())
    }

    pub(super) async fn complete(&self, parameters: HashMap<String, String>) -> Result<bool, ()> {
        let mut state = self.state.lock().await;
        let pending = state.pending.as_ref().ok_or(())?;
        if parameters.get("state") != Some(&pending.state) {
            return Err(());
        }
        let pending = state.pending.take().unwrap();
        if parameters.contains_key("error") {
            state.message = Some("Google sign-in was cancelled or denied.".into());
            return Ok(false);
        }
        let result = self
            .exchange(
                parameters
                    .get("code")
                    .map(String::as_str)
                    .unwrap_or_default(),
                pending,
            )
            .await;
        match result {
            Ok((credentials, calendars)) => {
                state.credentials = Some(credentials);
                state.calendars = calendars;
                state.event_cache = None;
                state.message = None;
                tracing::info!(action = "google_connected");
                Ok(true)
            }
            Err(error) => {
                state.message = Some(error);
                tracing::warn!(action = "google_sign_in_failed");
                Ok(false)
            }
        }
    }

    pub(super) async fn exchange(
        &self,
        code: &str,
        pending: Pending,
    ) -> Result<(Credentials, Vec<GoogleCalendar>), String> {
        if code.is_empty() {
            return Err("Google did not return an authorization code.".into());
        }
        let config = self
            .config
            .as_ref()
            .ok_or("Missing Google configuration.")?;
        let response = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("client_secret", config.client_secret.as_str()),
                ("code", code),
                ("code_verifier", &pending.verifier),
                ("redirect_uri", &pending.redirect_uri),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|_| "Google token exchange unavailable. Try connecting again.")?;
        if !response.status().is_success() {
            return Err(
                "Google rejected sign-in. Check Desktop OAuth credentials and reconnect.".into(),
            );
        }
        let tokens: Tokens = response
            .json()
            .await
            .map_err(|_| "Invalid Google token response.")?;
        let mut credentials = Credentials {
            client_id: config.client_id.clone(),
            access_token: tokens.access_token,
            refresh_token: tokens
                .refresh_token
                .ok_or("Google did not grant offline access. Reconnect and approve access.")?,
            expires_at: chrono::Utc::now().timestamp() + tokens.expires_in,
            scope: tokens.scope.unwrap_or_else(|| SCOPE.into()),
            name: String::new(),
            email: String::new(),
            picture: String::new(),
        };
        let calendars = self.discover(&mut credentials).await?;
        if let Ok(response) = self
            .client
            .get("https://openidconnect.googleapis.com/v1/userinfo")
            .bearer_auth(&credentials.access_token)
            .send()
            .await
            && response.status().is_success()
            && let Ok(profile) = response.json::<serde_json::Value>().await
        {
            credentials.name = profile["name"].as_str().unwrap_or_default().into();
            credentials.email = profile["email"].as_str().unwrap_or_default().into();
            credentials.picture = profile["picture"]
                .as_str()
                .filter(|url| url.starts_with("https://"))
                .unwrap_or_default()
                .into();
        }
        store_credentials(self.slot.clone(), Some(credentials.clone())).await?;
        Ok((credentials, calendars))
    }

    pub async fn cancel(&self) {
        let mut state = self.state.lock().await;
        state.pending = None;
        state.message = Some("Sign-in cancelled. Existing account kept.".into());
    }

    pub async fn disconnect(&self) -> Result<bool, String> {
        let mut state = self.state.lock().await;
        self.initialize(&mut state).await;
        // Delete the local secret first, even when Google is unreachable.
        store_credentials(self.slot.clone(), None).await?;
        let credentials = state.credentials.take();
        state.pending = None;
        state.calendars.clear();
        state.event_cache = None;
        state.message = Some("Google disconnected from this machine.".into());
        if let Some(credentials) = credentials {
            let revoked = self
                .client
                .post("https://oauth2.googleapis.com/revoke")
                .form(&[("token", credentials.refresh_token)])
                .send()
                .await;
            if !revoked.is_ok_and(|response| response.status().is_success()) {
                state.message = Some("Disconnected locally. Remove Calendar access in your Google account to revoke the remote grant.".into());
            }
        }
        tracing::info!(action = "google_disconnected");
        Ok(true)
    }

    pub async fn forget_local(&self) -> Result<(), String> {
        store_credentials(self.slot.clone(), None).await?;
        let mut state = self.state.lock().await;
        state.credentials = None;
        state.calendars.clear();
        state.event_cache = None;
        Ok(())
    }
}
