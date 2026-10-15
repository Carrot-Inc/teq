// A literal where an opaque type is expected inside its companion, which sees the alias through
// (indigo's `Volume.apply`), and pattern alternatives going on over a line's end after a trailing `|`
// or before a leading one (dotty's scanner inserts no newline between `case` and its `=>`).
package audio

opaque type Volume = Double
object Volume:
  inline def apply(volume: Double): Volume =
    if (volume < 0) 0 else if (volume > 1) 1 else volume
  val Min: Volume = Volume(0)
  val Max: Volume = Volume(1)
  extension (v: Volume) def toDouble: Double = v

enum Code:
  case A, B, C, D, E
def kind(c: Code): String = c match
  case Code.A | Code.B |
      Code.C => "first"
  case Code.D
      | Code.E => "last"

@main def run(): Unit =
  println(List(Volume(2), Volume(-1), Volume(0.5), Volume.Max).map(v => (v.toDouble * 10).toInt))
  println(Code.values.map(kind).mkString(","))
