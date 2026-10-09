// A constructor call whose second argument list is cut: the first list is complete, and its
// arity error stands beside the missing parenthesis.
class C(a: Int, b: Int)(c: Int)
val instance = new C(1, 2, 3)(4
