package events
trait EventTypes:
  type ReactEvent = String
  final type ReactKeyboardEvent = Int
  def describe(e: ReactEvent): String = s"event $e"
trait Core extends EventTypes
