// The calls come first among the files and in the larger file, so that the typer meets them
// before the class they call: `getOrElse` and `map` expand at the call, the expansion asks for
// the result type of `isEmpty`, which is inferred, and the body of `isEmpty` is typed on that
// demand. `this` in it is the instance `isEmpty` is called on, not the receiver of the call that
// asked: read as the receiver, `isEmpty` would name a local of `first` from its own body.
package demanded

object Uses:
  def first(): String =
    val opt = new Optional[String]("hello")
    opt.getOrElse("world")

  def second(): String =
    val none = new Optional[String](null)
    none.getOrElse("fallback")

  def third(): String =
    val opt = new Optional[String]("hello")
    opt.map(_ + " world").toString

  def fourth(): String =
    val none = new Optional[String](null)
    none.map(_ + " world").toString

  def fifth(): Int =
    val box = new Counted(3)
    box.twice + box.size

@main def run(): Unit =
  println(Uses.first())
  println(Uses.second())
  println(Uses.third())
  println(Uses.fourth())
  println(Uses.fifth())
  println(new Optional[String]("direct").isEmpty)
  println(new Optional[String](null).isEmpty)
