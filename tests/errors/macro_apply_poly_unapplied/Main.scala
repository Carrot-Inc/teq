// expect: Main.scala:5:11: error: method f in object M does not take parameters
// expect: Main.scala:6:11: error: constructor Gen in class Gen does not take parameters
// expect: 2 errors found
@main def main() =
  println(Mac.test)
  println(Mac.ctor)
