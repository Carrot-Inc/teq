package probe

class G extends (Int => Int) { def apply(n: Int): Int = n + 1 }
class H extends G
