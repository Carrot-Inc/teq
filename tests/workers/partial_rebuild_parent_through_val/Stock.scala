package inc
class Api:
  class Request(val n: Int)
object Stock:
  val api = { val made = new Api; made }
  val seed = api.toString.length
