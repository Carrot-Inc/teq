// An `inline if` over a transparent macro's result reduces once the macro runs: its literal result is a
// constant (`Literal`'s `ConstantType`), as the condition's ConstantTree reads it. scalac prints the lines of the
// .expected file.
@main def run(): Unit =
  L.log(3, "shown")
  L.log(1, "hidden")
