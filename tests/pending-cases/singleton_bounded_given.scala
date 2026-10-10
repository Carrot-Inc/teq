// A given searched for a type parameter bounded by `Quotes & Singleton` takes the `Quotes` in scope by its
// singleton type, a stable parameter's (scalac infers `Q` as `x$1.type`), as Iron's `reflectUtil` does in every
// macro of its constraints; teq reports "no given instance of type Quotes & Singleton was found for parameter q".
import scala.quoted.*

class Util[Q <: Quotes & Singleton](using val q: Q):
  def name: String = "util"

transparent inline def util[Q <: Quotes & Singleton](using inline q: Q): Util[Q] = new Util[Q]

def check(x: Expr[Int])(using Quotes): Expr[Int] =
  val u = util
  import u.*
  '{ $x + 1 }

@main def main(): Unit = println("ok")
