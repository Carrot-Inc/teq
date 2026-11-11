// An array spread into a Java method's varargs where Predef's wildcard import is unimported (`import scala.Predef.Set`):
// typedWildcardStarArgExpr types the array as the repeated argument, no conversion asked for, and ElimRepeated passes
// a Java method the array itself, a primitive array boxed into one of the parameter's class (pos/i4785).
import scala.Predef.Set
import java.nio.file.Paths

class Spread:
  def path(ys: Array[String]) = Paths.get("Hello", ys*)
  def format(xs: Array[String]) = String.format("%s-%s", xs*)
  def list(xs: Array[String]) = java.util.List.of(xs*)
  def boxed(ints: Array[Int]) = java.util.Arrays.asList(ints*)
