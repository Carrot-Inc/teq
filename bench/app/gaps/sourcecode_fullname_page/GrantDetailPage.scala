package meridian.frontend.page.grant

import meridian.core.*
import meridian.core.Codecs.given
import meridian.core.http.{Request, Response}
import meridian.web.*
import meridian.model.*
import meridian.model.api.*
import meridian.frontend.main.*
import meridian.frontend.apiclient.*
import cats.syntax.all.*
import meridian.frontend.component.widget.*
import meridian.frontend.component.grant.*
import meridian.frontend.main.actions.*
import meridian.frontend.main.state.*

final case class GrantDetailProps(id: StepperId, actions: Actions)
val grantDetailPage: FC[GrantDetailProps] = FC[GrantDetailProps]: props =>
  val slice = useStore(Environment.store)(_.grant)
  val item = slice.items.get(props.id)
  val (confirm, setConfirm) = useState(false)
  useEffect(props.id.raw)(props.actions.run(GrantActions.load(props.id)))
  val body = item match
    case None => spinner(SpinnerProps(true))
    case Some(value) =>
      div(cls := "decoration-teal-950 whitespace-nowrap text-sm mx-28",
        grantCard(GrantCardProps(value, true, _ => Eff.unit)),
        keyValue(KeyValueProps(List(pair("id", value.id.raw.toString), pair("fields", value.productArity.toString)))),
        Tags.button(cls := "max-lg:active:flex-col py-8 gap-y-24", onClick --> setConfirm(true), "remove"),
        modal(ModalProps("remove?", confirm, setConfirm(false), Some(dangerButton("remove", props.actions.run(GrantActions.remove(props.id)) *> setConfirm(false)))))(p("this cannot be undone")),
        slice.draft.ifDefinedNode(draft => grantEditor(GrantEditorProps(draft, slice.errors, d => props.actions.run(GrantActions.edit(d)), props.actions.run(GrantActions.save), props.actions.run(GrantActions.discard)))))
  pageShell(PageShellProps(s"${props.id.raw}", props.actions, List("home", "grant", "detail")))(
    body,
    fc:
      small(cls := ("ring-pink-200 capitalize", tw"2xl:inline"), s"loaded ${slice.count} of ${slice.order.length}"))
