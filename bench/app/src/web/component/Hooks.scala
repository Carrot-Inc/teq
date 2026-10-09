package meridian.web.component

import meridian.core.effect.{Eff, Task}
import meridian.core.optics.Lens
import meridian.web.vdom.Renderer

/** Hooks over the renderer's slots: a slot per call, in call order, per mounted component. */
object Hooks:
  final class RefHandle[A](var current: A):
    def set(a: A): Task[Unit] = Eff.succeed { current = a }
    def get: Task[A] = Eff.succeed(current)
    def currentOption: Option[A] = Option(current)

  def useState[S](initial: S): (S, S => Task[Unit]) =
    val context = Renderer.current
    val (index, value) = Renderer.slotValue(context, initial)
    val path = context.path
    (value.asInstanceOf[S], s => Eff.succeed(Renderer.setSlot(path, index, s)))

  def useStateWithUpdater[S](initial: S): (S, (S => S) => Task[Unit]) =
    val context = Renderer.current
    val (index, value) = Renderer.slotValue(context, initial)
    val path = context.path
    (value.asInstanceOf[S], f => Eff.succeed(Renderer.setSlot(path, index, f(Renderer.readSlot(path, index).asInstanceOf[S]))))

  def useStateL[S](initial: S): (S, LensSetter[S]) =
    val (state, update) = useStateWithUpdater(initial)
    (state, new LensSetter[S](update))

  final class LensSetter[S](update: (S => S) => Task[Unit]):
    def apply[T](lens: Lens[S, T])(value: T): Task[Unit] = update(lens.replace(value))
    def modify[T](lens: Lens[S, T])(f: T => T): Task[Unit] = update(lens.modify(f))

  def useRef[A](initial: A): RefHandle[A] =
    val (_, value) = Renderer.slotValue(Renderer.current, new RefHandle(initial))
    value.asInstanceOf[RefHandle[A]]

  def useEffect[D](deps: D)(effect: => Task[Unit])(using reuse: Reusability[D]): Unit =
    val context = Renderer.current
    val (index, previous) = Renderer.slotValue(context, Unset)
    val changed = previous match
      case Unset => true
      case old => !reuse.test(old.asInstanceOf[D], deps)
    if changed then
      Renderer.setSlotQuietly(context.path, index, deps)
      Renderer.schedule(effect)

  def useMountEffect(effect: => Task[Unit]): Unit = useEffect(())(effect)

  def useMemo[D, A](deps: D)(compute: => A)(using reuse: Reusability[D]): A =
    val context = Renderer.current
    val (index, previous) = Renderer.slotValue(context, Unset)
    previous match
      case (old, value) if reuse.test(old.asInstanceOf[D], deps) => value.asInstanceOf[A]
      case _ =>
        val value = compute
        Renderer.setSlotQuietly(context.path, index, (deps, value))
        value

  def useMountMemo[A](compute: => A): A = useMemo(())(compute)

  def useCallback[D](deps: D)(callback: => Task[Unit])(using reuse: Reusability[D]): Task[Unit] = useMemo(deps)(callback)

  def useStore[S, A](store: Store[S])(select: S => A): A =
    val context = Renderer.current
    val selected = select(store.current)
    val (_, _) = Renderer.slotValue(context, store.subscribe(context.path))
    selected

  def useToggle(initial: Boolean): (Boolean, Task[Unit]) =
    val (value, set) = useState(initial)
    (value, set(!value))

  def useCounter(initial: Int): (Int, Task[Unit], Task[Unit]) =
    val (value, set) = useState(initial)
    (value, set(value + 1), set(value - 1))

  def useId(): String = Renderer.current.path.hashCode.toHexString

  private object Unset

extension (renderer: Renderer.type)
  def setSlotQuietly(path: String, index: Int, value: Any): Unit = renderer.setSlot(path, index, value, quiet = true)
