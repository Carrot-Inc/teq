// A named tuple as a map's value: a plain tuple added where the named one is expected, `toTuple`
// over an `Option` of it unzipped, and a field read by name.

final case class PushId(value: String)

object Main:
  type Tracked = (pushId: PushId, pushedTime: Long)
  def main(args: Array[String]): Unit =
    val now = 1234L
    var pushes: Map[Int, Tracked] = Map.empty
    pushes = pushes + (1 -> (PushId("a"), now))
    pushes = pushes + (2 -> (pushId = PushId("b"), pushedTime = 99L))
    val (ids, times) = pushes.get(1).map(_.toTuple).unzip
    println(ids)
    println(times)
    println(pushes.get(2).exists(_.pushId == PushId("b")))
    println(pushes.get(3).map(_.toTuple).unzip)
