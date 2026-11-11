// expect: extends enum Choice, but extending enums is prohibited.
// expect: 1 error found
enum Choice:
  case One
object Use:
  def make = new Choice { def ordinal = 0 }
