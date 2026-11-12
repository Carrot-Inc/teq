// jars: ctorinit-lib
// A macro that constructs a class and a trait's subclass of a scalac-compiled
// jar (tests/support/ctorinit_lib.scala) whose constructors' statements call
// a method reading a val the template defines before them: the construction
// runs each of the template's statements once, in order (izumi-reflect's
// `Tag` macro, whose boopickle `IdentMap3Plus` reads `maxDepth` so).
import scala.quoted.*
import ctorinitlib.*

object Probe:
  inline def recorded: String = ${ recordedImpl }
  def recordedImpl(using Quotes): Expr[String] =
    Expr(s"${Recorder("a", "b", "c").log} ${Counter().seen.mkString(",")}")
