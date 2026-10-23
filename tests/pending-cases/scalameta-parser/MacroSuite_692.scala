// Program 692 of tests/fix/scalameta, from scalameta 4.17.4's MacroSuite (line 183, "macro-quote-this-within-splice:
// ${ 'this }"): scalac 3.8.4's parser reads it as scalameta does, `${'{this}}`, and teq's otherwise. The expectation
// is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 42
//     Defn.Object 0 41
//       Term.Name 7 11
//       Template 12 41
//         Template.Body 12 41
//           Defn.Def 16 39
//             Term.Name 20 21
//             Term.Block 24 39
//               Term.SplicedMacroExpr 26 37
//                 Term.Block 28 37
//                   Term.QuotedMacroExpr 30 35
//                     Term.This 31 35
//                       Name.Anonymous 31 31
object W692 {
  def w = {
$ { 'this }
}
}
