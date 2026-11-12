// The divergence check of a given search compares the covering sets of two goals (dotty's
// `Implicits.checkDivergence`: `tp.coveringSet != ptCoveringSet`, `Types.CoveringSetAccumulator`): `via` is asked for
// `TC[Box[Q]]` while it is open for `TC[P]`, a larger goal of other classes, which is no divergence. `P`, `Box` and
// `Q` stand 64 class indices apart (an object takes one), where a mask of the indices modulo 64 holds the two sets
// for one. scalac 3.8.4 prints `via via R`.
trait TC[A]:
  def name: String
trait Rel[A, B]
final class P
object PadA0
object PadA1
object PadA2
object PadA3
object PadA4
object PadA5
object PadA6
object PadA7
object PadA8
object PadA9
object PadA10
object PadA11
object PadA12
object PadA13
object PadA14
object PadA15
object PadA16
object PadA17
object PadA18
object PadA19
object PadA20
object PadA21
object PadA22
object PadA23
object PadA24
object PadA25
object PadA26
object PadA27
object PadA28
object PadA29
object PadA30
object PadA31
object PadA32
object PadA33
object PadA34
object PadA35
object PadA36
object PadA37
object PadA38
object PadA39
object PadA40
object PadA41
object PadA42
object PadA43
object PadA44
object PadA45
object PadA46
object PadA47
object PadA48
object PadA49
object PadA50
object PadA51
object PadA52
object PadA53
object PadA54
object PadA55
object PadA56
object PadA57
object PadA58
object PadA59
object PadA60
object PadA61
object PadA62
final class Box[A]
object PadB0
object PadB1
object PadB2
object PadB3
object PadB4
object PadB5
object PadB6
object PadB7
object PadB8
object PadB9
object PadB10
object PadB11
object PadB12
object PadB13
object PadB14
object PadB15
object PadB16
object PadB17
object PadB18
object PadB19
object PadB20
object PadB21
object PadB22
object PadB23
object PadB24
object PadB25
object PadB26
object PadB27
object PadB28
object PadB29
object PadB30
object PadB31
object PadB32
object PadB33
object PadB34
object PadB35
object PadB36
object PadB37
object PadB38
object PadB39
object PadB40
object PadB41
object PadB42
object PadB43
object PadB44
object PadB45
object PadB46
object PadB47
object PadB48
object PadB49
object PadB50
object PadB51
object PadB52
object PadB53
object PadB54
object PadB55
object PadB56
object PadB57
object PadB58
object PadB59
object PadB60
object PadB61
object PadB62
final class Q
final class R
object G:
  given relP: Rel[P, Box[Q]] = new Rel[P, Box[Q]] {}
  given relB: Rel[Box[Q], R] = new Rel[Box[Q], R] {}
  given leaf: TC[R] = new TC[R] { def name = "R" }
  given via[A, B](using r: Rel[A, B], tc: TC[B]): TC[A] = new TC[A] { def name = "via " + tc.name }

import G.given

@main def run(): Unit =
  println(summon[TC[P]].name)
