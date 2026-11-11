// A type member bounded by a type lambda (`type L <: [X, Y] =>> Any`) overridden by an alias to one (`type L = [X,
// Y] =>> Map[Y, X]`): scalac 3.8.4 accepts it and applies `h.L[Int, String]` as `Map[String, Int]`. teq reports
// "error overriding type L in trait H with bounds[X, Y] <: Any; type L, which equals [X, Y] =>> Map[Y, X] has
// incompatible type" (master the same). The closure adversary of the pass on 97f0487ac (round 24).
trait H { type L <: [X, Y] =>> Any }
trait H2 extends H { type L = [X, Y] =>> Map[Y, X] }
def f(h: H2)(x: h.L[Int, String]): Map[String, Int] = x

@main def run(): Unit = println(f(new H2 {})(Map("a" -> 1)))
