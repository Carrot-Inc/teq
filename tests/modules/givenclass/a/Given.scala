package mixed
trait T
given G: T with { class K(val n: Int) { def get: Int = n * 2 } }
