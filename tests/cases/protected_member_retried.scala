// A protected member the site may not access is retried by an extension method and by a conversion before the
// access is refused, as a private one is: from outside the class the conversion or the extension gives the name;
// inside a subclass the member itself is taken.
import scala.language.implicitConversions

class Guarded:
  protected def stamp: String = "member"
  protected def mark(n: Int): String = s"member$n"

class Rich { def stamp: String = "conversion" }
given Conversion[Guarded, Rich] = _ => new Rich

extension (g: Guarded) def mark(n: Int): String = s"extension$n"

class Inside extends Guarded:
  def own: String = this.stamp + " " + mark(1)
  def other(o: Inside): String = o.stamp

@main def run(): Unit =
  val g = new Guarded
  println(g.stamp)
  println(g.mark(2))
  val i = new Inside
  println(i.own + " " + i.other(i))
