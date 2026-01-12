export type UpdateDialogRequest =
  | {
      kind: "message";
      title: string;
      message: string;
      confirmText?: string;
    }
  | {
      kind: "confirm";
      title: string;
      message: string;
      confirmText?: string;
      cancelText?: string;
    }
  | {
      kind: "progress";
      title: string;
      message: string;
      progress: number; // 0-100
      stage: "downloading" | "installing";
    }
  | {
      kind: "error";
      title: string;
      message: string;
      details: string;
      confirmText?: string;
    };

type UpdateDialogHandler = (request: UpdateDialogRequest) => Promise<boolean | void>;

let handler: UpdateDialogHandler | null = null;

export function setUpdateDialogHandler(next: UpdateDialogHandler | null): void {
  handler = next;
}

export async function showUpdateMessage(title: string, message: string): Promise<void> {
  if (handler) {
    await handler({ kind: "message", title, message });
    return;
  }

  window.alert(`${title}\n\n${message}`);
}

export async function showUpdateConfirm(
  title: string,
  message: string,
  options?: { confirmText?: string; cancelText?: string }
): Promise<boolean> {
  if (handler) {
    const result = await handler({
      kind: "confirm",
      title,
      message,
      confirmText: options?.confirmText,
      cancelText: options?.cancelText,
    });
    return Boolean(result);
  }

  return window.confirm(`${title}\n\n${message}`);
}

export async function showUpdateProgress(
  title: string,
  message: string,
  progress: number,
  stage: "downloading" | "installing"
): Promise<void> {
  if (handler) {
    await handler({ kind: "progress", title, message, progress, stage });
  }
}

export async function showUpdateError(
  title: string,
  message: string,
  details: string
): Promise<void> {
  if (handler) {
    await handler({ kind: "error", title, message, details });
    return;
  }

  window.alert(`${title}\n\n${message}\n\nDetails:\n${details}`);
}
