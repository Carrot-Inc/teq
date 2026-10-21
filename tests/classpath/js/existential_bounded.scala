// jars: fixtures
// targets: js interp jvm
// chimney-macro-commons' `Existentials` shape (the gate of `bb04d0aaf`, fix.exist): a macro builds a bounded existential
// and maps it at expansion, so teq types the library's bodies from its pickles. A library class whose parameter takes a
// kind of its own (`F[_ >: L <: U]`) has its parameters' bounds read for a kind without completing it
// (`TypeParamsCompleter`): completed amid the loading of an alias naming it (`Existential.UpperBounded`), its case class
// lost the sealed parent's base type (`found Impl[L, U, F, A], required Bounded[L, U, F]`).
import fix.exist.ExbMac

object Main:
  def main(args: Array[String]): Unit = println(ExbMac.bounded(3))
