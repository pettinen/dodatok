type Alert = import("$lib/errors").Alert;
type User = import("$lib/types").User;


declare namespace App {
  interface Locals {
    csrfToken: string | null;
    errors: Set<Alert>;
    user: User | null;
  }

  type Platform = Record<string, never>;

  type Session = Locals;

  type Stuff = Record<string, never>;
}
