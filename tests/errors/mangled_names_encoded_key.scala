// A name of the empty package is found by its encoded key, which decoding never reaches (dotty's PackageScope keys
// by name.mangled, NameTransformer.encode): `$u0041` encodes to itself, not to `A`, as scalac 3.8.4 refuses it.
object A:
  val value = 42
object Main:
  def main(args: Array[String]): Unit = println($u0041.value)
// expect: 6:49: error: not found: $u0041
// expect: 1 error found
