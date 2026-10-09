// A union of 33 literals is a Singleton bound's argument: only the bounds followed from a
// parameter or a member count against the check's depth, not the parts of one type.
class S[T <: Singleton]
val s = new S[1|2|3|4|5|6|7|8|9|10|11|12|13|14|15|16|17|18|19|20|21|22|23|24|25|26|27|28|29|30|31|32|33]
@main def run(): Unit = println("ok")
