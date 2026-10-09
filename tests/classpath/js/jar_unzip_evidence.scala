// jars: predef-lib
// A jar's `unzip` and `unzip3`, applied to their type arguments and `Predef.$conforms`, go to the
// std's, which take neither (utest's `Formatter` unzips so).
@main def run(): Unit =
  println(predeflib.Unzips.split(List((1, "a"), (2, "b"))))
  println(predeflib.Unzips.split3(Vector((1, "a", true), (2, "b", false))))
