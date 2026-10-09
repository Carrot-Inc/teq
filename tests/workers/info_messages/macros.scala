import scala.quoted.*

// A macro's runs give info messages, two each, at sites in every file, some of them the same
// inline method's sites nested in another's: the build prints them in the order of the sites,
// whichever worker ran which.
object Info:
  def say(e: Expr[String])(using Quotes): Expr[Unit] =
    val s = e.valueOrAbort
    quotes.reflect.report.info(s"first $s")
    quotes.reflect.report.info(s"second $s")
    '{ () }

inline def say(inline s: String): Unit = ${ Info.say('s) }
inline def twice(inline s: String): Unit =
  say(s + " inner-1")
  say(s + " inner-2")
