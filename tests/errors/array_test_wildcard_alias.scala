// expect: 27:30: warning: the type test for Array[? <: T] cannot be checked at runtime
// expect: 28:30: warning: the type test for Array[Array[? <: T]] cannot be checked at runtime
// absent: 20:27: warning
// absent: 21:27: warning
// absent: 22:27: warning
// absent: 23:27: warning
// absent: 24:27: warning
// expect: 2 warnings found, errors under --werror
// teq: --target jvm --werror
// A wildcard element's bound that is a class is one the JVM's test of an array sees, under any spelling: teq is
// silent on the five lines marked absent, and on the two beside them whose bound is written by its class. scalac
// 3.8.4 WARNS of those five (E092 "the type test for Array[? <: AnyRef] cannot be checked at runtime because it
// refers to an abstract type member or type parameter") and not of the two: it asks whether the bound is abstract
// of the bound's symbol as written, the alias's, without following the alias
// (compiler/src/dotty/tools/dotc/transform/TypeTestsCasts.scala 64 `isAbstract`, asked at 140-141), while the test
// it writes is exact (`instanceof Object[]`). A difference from scalac that teq keeps. Both tell the bound that IS
// a type parameter (the two lines expected).
class Box
type BoxAlias = Box
def w1(x: Any): Boolean = x.isInstanceOf[Array[? <: AnyRef]]
def w2(x: Any): Boolean = x.isInstanceOf[Array[? <: String]]
def w3(x: Any): Boolean = x.isInstanceOf[Array[? <: BoxAlias]]
def w4(x: Any): Boolean = x.isInstanceOf[Array[Array[? <: AnyRef]]]
def w5(x: Any): Boolean = x.isInstanceOf[Array[Array[? <: BoxAlias]]]
def w6(x: Any): Boolean = x.isInstanceOf[Array[? <: Object]]
def w7(x: Any): Boolean = x.isInstanceOf[Array[Array[? <: Box]]]
def t1[T](x: Any): Boolean = x.isInstanceOf[Array[? <: T]]
def t2[T](x: Any): Boolean = x.isInstanceOf[Array[Array[? <: T]]]
