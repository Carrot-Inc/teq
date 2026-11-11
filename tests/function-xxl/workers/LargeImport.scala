// An unused import of a synthetic function class past every arity a program completes, typed by the workers of
// Multi.scala's objects: no worker allocates for an arity no completion reaches (tests/function-xxl.sh).
package largeimport
object LargeImport {
  def f(): Int = {
    import scala.ContextFunction10000000
    1
  }
}
