package inc
class Api:
  class Request
object Stock:
  val api = { val size = Counter.sized(3); if size > 0 then new Api else new Api }
