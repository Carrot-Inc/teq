// jars: scala-library
// teq: --deprecation --werror --wconf origin=scala.collection.IterableOnceOps.toStream:s
// A warning's origin names the class scala-library declares the member in: a build that silences `toStream`'s
// warning by it passes under `--werror`, with scala-library's jar on the class path beside the std too.
object Main {
  def main(a: Array[String]): Unit = println(List(4, 5).toStream.head)
}
