use super::*;

pub(super) fn event_surface(color: u32) -> u32 {
    let channel = |shift: u32| (((color >> shift) & 255u32) as f32 * 0.30 + 25.0 * 0.70) as u32;
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

pub(super) fn icon_button(identifier: &'static str, path: &'static str) -> Stateful<Div> {
    div()
        .id(identifier)
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, context| {
            context.stop_propagation()
        })
        .size(px(30.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .cursor_pointer()
        .text_color(rgb(MUTED))
        .hover(|style| style.bg(rgb(0xffffff).opacity(0.07)))
        .on_click(move |_, _, _| calendar_telemetry::track("control_clicked", identifier))
        .child(svg().path(path).size(px(18.0)).text_color(rgb(0xbdbdbd)))
}

pub(super) fn button(identifier: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(identifier)
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, context| {
            context.stop_propagation()
        })
        .on_click(move |_, _, _| calendar_telemetry::track("control_clicked", identifier))
        .flex()
        .items_center()
        .px_2()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .text_size(px(12.0))
        .hover(|style| style.bg(rgb(0xffffff).opacity(0.07)))
        .child(label)
}

pub(super) fn calendar_toggle(
    identifier: SharedString,
    title: SharedString,
    color: u32,
    visible: bool,
) -> Stateful<Div> {
    div()
        .id(identifier)
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, context| {
            context.stop_propagation()
        })
        .h(px(30.0))
        .px_2()
        .flex()
        .items_center()
        .gap_3()
        .rounded_md()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x303030)))
        .child(
            div()
                .size(px(12.0))
                .rounded_sm()
                .bg(rgb(if visible { color } else { 0x555555 })),
        )
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(if visible { 0xcccccc } else { MUTED }))
                .truncate()
                .child(title),
        )
}

pub(super) fn field(label: &'static str, input: impl IntoElement) -> Div {
    div()
        .mb_3()
        .child(
            div()
                .text_size(px(9.0))
                .text_color(rgb(MUTED))
                .mb_1()
                .child(label),
        )
        .child(input)
}
