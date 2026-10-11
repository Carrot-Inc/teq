// A summonFrom case that names no binder (`case _: Marker`) binds the given it finds, as scalac's
// typer rewrites it (`case _$1: Marker`, Typer.typedMatch): the expansion evaluates the given.
import scala.compiletime.summonFrom

trait Marker

given preferred: Marker =
  println("init preferred")
  new Marker {}

inline def selected: Int = summonFrom {
  case _: Marker => 1
  case _ => 2
}

inline def guarded(inline b: Boolean): Int = summonFrom {
  case _: Marker if b => 3
  case _ => 4
}

@main def run(): Unit =
  println("before")
  println(selected)
  println(guarded(true))
  println(guarded(false))
