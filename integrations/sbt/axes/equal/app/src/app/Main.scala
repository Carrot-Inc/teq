package app

object Main {
  def main(args: Array[String]): Unit = println(s"${BuildInfo.name} ${args.mkString(" ")}".trim)
}
