//> using dep com.softwaremill.magnolia1_3::magnolia:1.3.18
// jars: magnolia
// A sealed trait derived through magnolia1 at three sites each, from three places: inside a package object (op.po.PS),
// top-level in that package in another file (op.po.TS), and in a package with no package object (oq.QS). The
// expansions of one shape at several sites are written once as an outlined function, named by a hash of the classes
// and members it names: a class of the package object is owned by the module `op.po.package`, as scalac's, so the
// shapes naming PA, PB and PC are named by that owner. The output is scalac's: each sealed trait's owner and its
// subtypes, each case class's labels.
import oz.Show

object Uses1:
  def run(): Unit =
    println(Show.derived[op.po.PS].show(op.po.PA(1)))
    println(Show.derived[op.po.TS].show(op.po.TA(1)))
    println(Show.derived[oq.QS].show(oq.QA(1)))

object Uses2:
  def run(): Unit =
    println(Show.derived[op.po.PS].show(op.po.PB("b")))
    println(Show.derived[op.po.TS].show(op.po.TB("b")))
    println(Show.derived[oq.QS].show(oq.QB("b")))

object Uses3:
  def run(): Unit =
    println(Show.derived[op.po.PS].show(op.po.PC))
    println(Show.derived[op.po.TS].show(op.po.TC))
    println(Show.derived[oq.QS].show(oq.QC))

object Main:
  def main(args: Array[String]): Unit =
    Uses1.run()
    Uses2.run()
    Uses3.run()
