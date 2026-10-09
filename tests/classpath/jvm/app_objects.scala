// jars: scala-library
// `scala.App`'s objects and classes: the body runs when the object is made (Scala 3 has no
// `delayedInit` of the body), and `App.main` stores the arguments in the implementing class's field
// `scala$App$$_args` (master threw `AbstractMethodError` at `scala$App$$_args_$eq`), then runs what
// `delayedInit` queued, which reads them; in the body they are not stored yet, as under scalac. With
// arguments and without, an object and a class. The expectation is scalac's run of `run`
// (`scala-cli run --main-class run`: the objects are mains too).
object NoArgs extends App:
  println("NoArgs body, args stored: " + (args != null))
object Delayed extends App:
  println("Delayed body")
  delayedInit(println("Delayed args: [" + args.mkString(",") + "]"))
  def shown: String = args.mkString(",")
class Runner extends App:
  println("Runner body, started: " + (executionStart > 0))
  delayedInit(println("Runner args: " + args.length))
  def shown: String = args.mkString("|")

@main def run(): Unit =
  NoArgs.main(Array.empty)
  Delayed.main(Array("a", "b"))
  println(Delayed.shown)
  Delayed.main(Array.empty)
  val r = Runner()
  r.main(Array("x", "y"))
  println(r.shown)
  r.main(Array("z"))
  println(r.shown)
