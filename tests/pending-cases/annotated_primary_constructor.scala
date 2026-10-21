// An annotation on a class's primary constructor, between the name and the parameters, as scalafix's
// AnnotationScopes fixture writes it: scalac accepts it, teq reports "expected 'extends' or a template body, found
// '@'" (tests/fix/REFERENCES.md).
class C @SuppressWarnings(Array("x")) (val a: Int)
@main def run(): Unit = println(new C(1).a)
