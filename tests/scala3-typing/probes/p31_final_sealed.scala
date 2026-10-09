final class F
class G extends F
sealed trait S
object Other:
  class Impl extends S
final trait FT
class Q extends FT
object M:
  final def f = 1
class N extends M
@main def run(): Unit = println(1)
