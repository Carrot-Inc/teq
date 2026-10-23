// Program 1169 of tests/fix/scalameta, from scalameta 4.17.4's TypeSuite (line 710, "scala36 type member context
// bounds 1"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found ':'". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 52
//     Defn.Object 0 51
//       Term.Name 7 12
//       Template 13 51
//         Template.Body 13 51
//           Defn.Trait 15 48
//             Type.Name 21 24
//             Type.ParamClause 24 24
//             Ctor.Primary 24 24
//               Name.Anonymous 24 24
//             Template 24 48
//               Template.Body 24 48
//                 Decl.Type 28 48
//                   Type.Name 33 36
//                   Type.ParamClause 37 37
//                   Type.Bounds 37 48
//                     Type.Select 39 48
//                       Term.Name 39 43
//                       Type.Name 44 48
object W1169 {
trait Foo:
  type Key : cats.Show

}
