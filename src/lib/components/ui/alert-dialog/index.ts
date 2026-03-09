import { AlertDialog as AlertDialogPrimitive } from 'bits-ui';
import Content from './alert-dialog-content.svelte';
import Description from './alert-dialog-description.svelte';
import Footer from './alert-dialog-footer.svelte';
import Header from './alert-dialog-header.svelte';
import Overlay from './alert-dialog-overlay.svelte';
import Title from './alert-dialog-title.svelte';

const Root = AlertDialogPrimitive.Root;
const Trigger = AlertDialogPrimitive.Trigger;
const Action = AlertDialogPrimitive.Action;
const Cancel = AlertDialogPrimitive.Cancel;

export {
  Root,
  Trigger,
  Action,
  Cancel,
  Content,
  Description,
  Footer,
  Header,
  Overlay,
  Title,
  Root as AlertDialog,
  Trigger as AlertDialogTrigger,
  Action as AlertDialogAction,
  Cancel as AlertDialogCancel,
  Content as AlertDialogContent,
  Description as AlertDialogDescription,
  Footer as AlertDialogFooter,
  Header as AlertDialogHeader,
  Overlay as AlertDialogOverlay,
  Title as AlertDialogTitle
};
