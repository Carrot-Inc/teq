trait Greets { def hello: String }
case class GA() extends Greets { def hello = "ga" }
case class GB() extends Greets { def hello = "gb" }
def hello(x: GA | GB) = x.hello
@main def run(): Unit =
  val c = true
  val v = if c then 1 else "a"
  val w: Int | String = v
  println(w)
  val o1: Option[Long] = Some(1)
  println(o1)
  val nested: List[Int => Int] = List(x => x + 1)
  println(nested.head(1))
  println(hello(GA()))
  val i: Int = 3
  val b2 = if c then i else 2L
  val r = b2 match
    case _: Int => "int"
    case _: Long => "long"
  println(r)
