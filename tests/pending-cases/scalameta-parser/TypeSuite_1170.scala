// Program 1170 of tests/fix/scalameta, from scalameta 4.17.4's TypeSuite (line 732, "scala36 type member context
// bounds 2"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found ':'". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 71
//     Defn.Object 0 70
//       Term.Name 7 12
//       Template 13 70
//         Template.Body 13 70
//           Defn.Trait 15 67
//             Type.Name 21 24
//             Type.ParamClause 24 24
//             Ctor.Primary 24 24
//               Name.Anonymous 24 24
//             Template 24 67
//               Template.Body 24 67
//                 Decl.Type 28 67
//                   Type.Name 33 38
//                   Type.ParamClause 39 39
//                   Type.Bounds 39 67
//                     Type.Select 42 51
//                       Term.Name 42 46
//                       Type.Name 47 51
//                     Type.Select 53 66
//                       Term.Name 53 57
//                       Type.Name 58 66
object W1170 {
trait Foo:
  type Value : {cats.Show, cats.Traverse}

}
