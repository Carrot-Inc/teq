// expect: 14:11: error: value twice is not a member of Int
// expect: 1 error found
// The implicit scope of an extension's receiver is its widened type's (dotty's
// `ImplicitSearch.wildProto`, `argument.tpe.widen`): `y: O.x.type` is an `Int` there, whose scope
// `O` is not in, though `summon[O.x.type]` searches `O`.
trait IntOps:
  extension (i: Int) def twice: Int
object O:
  val x: Int = 3
  given ops: IntOps with
    extension (i: Int) def twice: Int = i * 2
@main def run(): Unit =
  val y: O.x.type = O.x
  println(y.twice)
