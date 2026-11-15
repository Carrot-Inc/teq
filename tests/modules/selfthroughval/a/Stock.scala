package inc
class Api:
  class Request(val n: Int)
trait Marker
trait Show[+T]:
  def show: String
object Stock:
  val api = { val made = new Api; made }
  val seed = api.getClass.getSimpleName.length
given firstShow: Show[Marker] = new Show[Marker] { def show = "marker" }
