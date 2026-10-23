// Program 1036 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 1693,
// "colon-eol-comment1"): scalac 3.8.4's parser reads it as scalameta does, `module object Foo { def foo: Int = ???
// def bar: Int = ???`, and teq's otherwise. The expectation is scalameta's tree under dialects.Scala3, which teq
// fix's tree answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 83
//     Defn.Object 0 82
//       Term.Name 7 12
//       Template 13 82
//         Template.Body 13 82
//           Defn.Object 15 79
//             Term.Name 22 25
//             Template 25 79
//               Template.Body 25 79
//                 Defn.Def 40 58
//                   Term.Name 44 47
//                   Type.Name 49 52
//                   Term.Name 55 58
//                 Defn.Def 61 79
//                   Term.Name 65 68
//                   Type.Name 70 73
//                   Term.Name 76 79
object W1036 {
object Foo:
  /*inline*/ def foo: Int = ???
  def bar: Int = ???

}
