import { addMonths, dateKey, eventColor, eventsOnDay } from "@calendar/calendar";
import { colors } from "@calendar/design-tokens";
import * as Haptics from "expo-haptics";
import { Platform, Pressable, Text, View } from "react-native";
import { textStyle } from "./components";
import type { MobileCalendarController } from "./use-mobile-calendar";

export function MobileMonth({ state }: { state: MobileCalendarController }) {
  const { selectedDate, setSelectedDate, weekStart, dates, events, touchStartRef } =
    state;

  return (
    <View
      className="px-4 pb-4"
      onTouchStart={(event) => {
        touchStartRef.current = event.nativeEvent.pageX;
      }}
      onTouchEnd={(event) => {
        const distance = event.nativeEvent.pageX - touchStartRef.current;

        if (Math.abs(distance) > 70)
          setSelectedDate((previous) => addMonths(previous, distance < 0 ? 1 : -1));
      }}
    >
      <View className="flex-row mb-2">
        {Array.from({ length: 7 }, (_, index) => (
          <Text
            key={index}
            style={[
              textStyle,
              {
                width: "14.285714%",
                textAlign: "center",
                color: colors.muted,
                fontSize: 11,
              },
            ]}
          >
            {["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"][(weekStart + index) % 7]}
          </Text>
        ))}
      </View>
      <View className="flex-row flex-wrap">
        {dates.map((date) => {
          const dailyEvents = eventsOnDay(events, date);

          return (
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={`${date}, ${dailyEvents.length} events`}
              accessibilityState={{ selected: date === selectedDate }}
              key={date}
              onPress={() => {
                setSelectedDate(date);

                if (Platform.OS !== "web")
                  void Haptics.selectionAsync().catch(() => {});
              }}
              style={{
                width: "14.285714%",
                height: 61,
                padding: 4,
                borderWidth: 1,
                borderRadius: 8,
                borderColor: date === selectedDate ? "#777" : "transparent",
                opacity: date.slice(0, 7) === selectedDate.slice(0, 7) ? 1 : 0.35,
              }}
            >
              <Text
                style={[
                  textStyle,
                  {
                    alignSelf: "center",
                    textAlign: "center",
                    fontSize: 17,
                    paddingHorizontal: 5,
                    borderRadius: 5,
                    backgroundColor: date === dateKey() ? colors.today : "transparent",
                  },
                ]}
              >
                {Number(date.slice(-2))}
              </Text>
              <View className="gap-0.5 mt-1">
                {dailyEvents.slice(0, 3).map((event) => (
                  <View
                    key={event.eventId}
                    style={{
                      height: 3,
                      borderRadius: 3,
                      backgroundColor: eventColor(event.color),
                    }}
                  />
                ))}
                {dailyEvents.length > 3 && (
                  <Text
                    style={[
                      textStyle,
                      {
                        fontSize: 8,
                        textAlign: "right",
                        color: colors.muted,
                      },
                    ]}
                  >
                    +{dailyEvents.length - 3}
                  </Text>
                )}
              </View>
            </Pressable>
          );
        })}
      </View>
    </View>
  );
}
