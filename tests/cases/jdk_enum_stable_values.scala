// A JDK enum's value is a static final field, which dotty's classfile parser reads as a stable value: it is a
// stable identifier in a pattern and has a singleton type (ClassfileParser's field flags).
import java.util.concurrent.TimeUnit
import java.math.RoundingMode

def unit(u: TimeUnit): String = u match
  case TimeUnit.SECONDS => "s"
  case TimeUnit.MILLISECONDS => "ms"
  case _ => "other"

def mode(m: RoundingMode): String = m match
  case RoundingMode.HALF_UP => "half up"
  case RoundingMode.FLOOR => "floor"
  case _ => "other"

@main def run(): Unit =
  println(unit(TimeUnit.SECONDS) + " " + unit(TimeUnit.MILLISECONDS) + " " + unit(TimeUnit.DAYS))
  println(mode(RoundingMode.HALF_UP) + " " + mode(RoundingMode.FLOOR) + " " + mode(RoundingMode.UP))
  val s: TimeUnit.SECONDS.type = TimeUnit.SECONDS
  val f: RoundingMode.FLOOR.type = RoundingMode.FLOOR
  println(s.toString + " " + f)
