// expect: 11:60: error: no given instance of type Int was found for parameter x
// expect: was not considered because it was not imported with `import given`.
// A search that finds nothing, with a given that `import Lib.*` does not bring (a wildcard brings no given): dotty's
// postscript names it (`ignoredInstanceNormalImport`, `hiddenImplicitNote`). scalac 3.8.4: E172 "No given instance of
// type Int was found for parameter x of method summon in object Predef" and "Note: given instance gx in object Lib was
// not considered because it was not imported with `import given`."
object Lib:
  given gx: Int = 1
object Main:
  import Lib.*
  def main(args: Array[String]): Unit = println(summon[Int])
