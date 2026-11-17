// teq: --std=scala-library
def uses(): Unit =
  class Local extends Dynamic { def selectDynamic(name: String): Int = 1 }
  println(new Local().slot)
// The feature `dynamics` is asked of a local class too, where the std is scala-library as on the lean std: the
// report is made once the class's check has its parents.
// expect: 3:9: error: extension of type scala.Dynamic needs to be enabled
// expect: 1 error found
