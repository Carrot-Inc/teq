package covariant
class Base[+A]
trait T extends Base[Int]
trait U extends Base[String]
class K extends T, U
