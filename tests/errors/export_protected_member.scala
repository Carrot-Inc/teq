// expect: 10:18: error: no eligible member secret at Public.this.library
// expect: Public.this.library.secret cannot be exported because it is not accessible
// expect: 11:33: error: value secret is not a member of Public
// expect: 2 errors found
// A protected member is reachable through a path only from a class deriving from its owner whose
// path is of that class (dotty's `isAccessibleFrom`, `isProtectedAccessOK`): not exported publicly.
class Library:
  protected def secret: Int = 7
class Public(val library: Library):
  export library.secret
@main def run(): Unit = println(new Public(new Library).secret)
