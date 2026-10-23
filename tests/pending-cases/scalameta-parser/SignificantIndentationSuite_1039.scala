// Program 1039 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 1768,
// "given-with-comment"): scalac 3.8.4's parser reads it as scalameta does, `given object extends Foo { def foo: Int
// = ??? def bar: Int = ???`, and teq's otherwise. The expectation is scalameta's tree under dialects.Scala3, which
// teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 92
//     Defn.Object 0 91
//       Term.Name 7 12
//       Template 13 91
//         Template.Body 13 91
//           Defn.Given 15 88
//             Name.Anonymous 21 21
//             Template 21 88
//               Init 21 24
//                 Type.Name 21 24
//                 Name.Anonymous 25 25
//               Template.Body 48 88
//                 Defn.Def 48 66
//                   Term.Name 52 55
//                   Type.Name 57 60
//                   Term.Name 63 66
//                 Defn.Def 70 88
//                   Term.Name 74 77
//                   Type.Name 79 82
//                   Term.Name 85 88
object W1039 {
given Foo with
   /* comment */  def foo: Int = ???
   def bar: Int = ???

}
