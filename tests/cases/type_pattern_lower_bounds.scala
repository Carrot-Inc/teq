// A type pattern's wildcards and type variables in contravariant positions of the scrutinee's
// class take the scrutinee's arguments as lower bounds, as scalac's GADT reasoning gives them
// (zio-streams' `ChannelExecutor.run` over a `Channel[Env]`): `Any` below a variable fixes it.
sealed trait Chan[-In, +Out]
final case class Read[In, Out, Out2](more: In => Chan[In, Out2], label: Out) extends Chan[In, Out2]
final case class Emit[Out](out: Out) extends Chan[Any, Out]

object Main:
  type Erased = Chan[Any, Any]
  def step(c: Erased, input: Any): Erased = c match
    case read: Read[?, ?, ?] => read.more(input)
    case Emit(o) => Emit(s"emitted $o")
  def named(c: Erased, input: Any): Erased = c match
    case read: Read[i, o, o2] =>
      val k: i => Chan[i, o2] = read.more
      k(input)
    case other => other
  def main(args: Array[String]): Unit =
    val r: Erased = Read[Any, String, Any]((x: Any) => Emit(x), "r")
    println(step(r, 5))
    println(step(Emit(1), 0))
    println(named(r, "n"))
