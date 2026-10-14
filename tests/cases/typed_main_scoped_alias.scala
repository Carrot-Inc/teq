// A typed @main method's proxy takes the formal parameter types of its typed signature, an alias of its
// object's scope among them (the review's `ScopedMain`): run without arguments, the missing one is the error.
object Config:
 type Count = Int
 @main def scoped(n: Count): Unit = println(n)
