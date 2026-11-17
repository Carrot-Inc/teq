package lib
abstract class K0 { val v: String; def read: String = v; val saw: String = "K0 saw " + v }
trait T1 extends K0 { override val v: String = "T1-trait" }
class K1 extends K0 with T1 { override val v: String = "K1-body"; def other: String = "other" }
