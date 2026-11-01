// jars: fixtures scala-library
// teq: --werror
// A library class's private constructor parameter passed on unchanged to its superclass's like-named one
// (tests/tasty/src/paramqual.scala's `PqLB(s: Int) extends PqLA(s)`, TASTy's `PARAMalias`): an `override val` through it
// is the ancestor's inherited accessor, which scalac 3.8.4 accepts with -Werror (`TreeUnpickler` reads `PARAMalias`
// as `SuperParamAlias`). teq does not carry the fact onto a loaded symbol and warns `cannot override val parameter
// value s in class PqLA`: pinned to the part after type-representation's landing, whose flag set carries
// `SuperParamAlias`.
class PqC2(override val s: Int) extends fix.shapes.PqLB(s)

@main def run(): Unit = println(PqC2(4).s)
