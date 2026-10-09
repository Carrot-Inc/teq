// A local transparent inline given summoned at the type of its right-hand side, which fixes
// the type argument a using parameter leaves open.
final class Conf[F <: Int]:
  def next: Conf[4] = new Conf[4]
inline def flagOf[F <: Int](using c: Conf[F]): Int = compiletime.constValue[F]
def run: Int =
  transparent inline given Conf[?] = new Conf[3].next
  flagOf
@main def main(): Unit = println(run)
