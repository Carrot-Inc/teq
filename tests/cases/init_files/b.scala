package p
import Log.t

val fileB1 = t("fileB1", 10)
val fileB2 = t("fileB2", fileB1 + 1)

@main def main(): Unit =
  println("main start")
  println(fileA2)
  println(fileALazy)
  fileAVar = 40
  println(fileAVar)
  println(fileAConst)
  println(fileA3)
  println(List(1, 3, 2).sorted)
  println(fileB2)
  println(c.fileC)
  println("main end")
