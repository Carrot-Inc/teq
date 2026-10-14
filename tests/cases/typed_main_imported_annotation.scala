// A program's own annotation class named `main`, imported, makes no proxy and asks no `FromString`: the
// object's `main` runs (the review's `ImportedMain`).
object Annotations:
 class main extends scala.annotation.StaticAnnotation
import Annotations.main
@main def ordinary(x: java.io.File): Unit = ()
object Main:
 def main(args: Array[String]): Unit = println("ok")
