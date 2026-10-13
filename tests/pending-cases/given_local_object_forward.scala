// A local structural given is a local object (dotty's `Parsers.givenDef`, a `ModuleDef`), in scope
// for the whole block as its search sees it: a summon before the definition takes it, made on its
// first read. teq reads its binding before the block declares it, as for any local object
// (tests/pending-cases/local_object_forward). scalac prints the lines of the .expected file.
trait QC { def q = "qc" }
object M:
  given QC with
    override def q = "m"
@main def run(): Unit =
  import M.given
  println(summon[QC].q)
  given QC()
  println(summon[QC].q)
