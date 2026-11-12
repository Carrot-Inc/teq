// jars: scala-library
// A JDK class the std stands for without a companion is to have the JDK class's module once a term names it, as dotty
// enters a Java class with its module (`SymbolLoaders.enterClassAndModule`) and a static member class's module beside
// it in its outer's (`ClassfileParser.enterOwnInnerClasses`): `Future.State.SUCCESS` through the std's `trait Future`,
// an import of `Future.State`, and `Map.Entry.comparingByKey` through the std's `trait Entry` in its `object Map`,
// whose result is to name the std's `Map.Entry` as the written type does.
import java.util.concurrent.Future.State

object Main:
  def main(args: Array[String]): Unit =
    println(java.util.concurrent.Future.State.SUCCESS)
    val failed: State = State.FAILED
    println(failed)
    val order: java.util.Comparator[java.util.Map.Entry[String, Integer]] = java.util.Map.Entry.comparingByKey[String, Integer]()
    println(order != null)
