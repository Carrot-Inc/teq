//> using scala 2.13.16
//> using dep org.scalameta::scalameta:4.17.4
// scalameta 4.17.4's tree of a file, as DumpLayout.scala prints teq's: a case's `scalameta.txt` is
// `scala-cli run --server=false tests/fix/support/MetaDump.scala -- <case>/input.scala > <case>/scalameta.txt`.
import scala.meta._

object MetaDump {
  def main(args: Array[String]): Unit = {
    implicit val dialect: Dialect = dialects.Scala3
    val text = new String(java.nio.file.Files.readAllBytes(java.nio.file.Paths.get(args(0))), "UTF-8")
    val tree = Input.VirtualFile(args(0), text).parse[Source].get
    def walk(t: Tree, depth: Int): Unit = {
      println("  " * depth + t.productPrefix + " " + t.pos.start + " " + t.pos.end)
      t.children.foreach(walk(_, depth + 1))
    }
    walk(tree, 0)
  }
}
