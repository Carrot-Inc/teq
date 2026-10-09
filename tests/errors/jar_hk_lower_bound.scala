// jars: fixtures
// expect: 6:62: error: no given instance of type HkCompiler[Option, G] was found for parameter c
// A jar method's `F2[x] >: F[x]` rules out an instance for another constructor (`HkId`).
import fix.hkb.HkStrm

@main def run(): Unit = println(HkStrm[Option, Int]().compile)
