package selection

import scala.compiletime.summonFrom

trait Marker

inline def selected: Int = summonFrom {
  case _: Marker => 1
  case _ => 2
}

def needs(using m: Marker): Int = 3
