trait S:
  def send(x: Int): Int
object S:
  enum Cat derives Show:
    case A, B
  final class Live extends S:
    def send(x: Int): Int = x + 1
