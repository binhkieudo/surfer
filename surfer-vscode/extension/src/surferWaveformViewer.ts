import * as vscode from 'vscode'
import * as path from 'path'

class Wavefile implements vscode.CustomDocument {
  uri: vscode.Uri
  constructor(uri: vscode.Uri) {
    this.uri = uri
  }
  dispose(): void {}
}

/** Maps an open-dialog `kind` to the Surfer `inject_message` JSON to fire once
 *  the user picks a file.  Returns `null` for unknown kinds. */
function openDialogMessage(kind: string, fileUri: vscode.Uri, webview: vscode.Webview): string | null {
  const url = webview.asWebviewUri(fileUri).toString()
  switch (kind) {
    case 'waveform_clear':
      return JSON.stringify({ LoadWaveformFileFromUrl: [url, 'Clear'] })
    case 'waveform_keep_available':
      return JSON.stringify({ LoadWaveformFileFromUrl: [url, 'KeepAvailable'] })
    case 'waveform_keep_all':
      return JSON.stringify({ LoadWaveformFileFromUrl: [url, 'KeepAll'] })
    case 'command_file':
      return JSON.stringify({ LoadCommandFileFromUrl: url })
    default:
      return null
  }
}

export class SurferWaveformViewerEditorProvider
  implements vscode.CustomReadonlyEditorProvider<Wavefile>
{
  public static register(context: vscode.ExtensionContext): vscode.Disposable {
    const provider = new SurferWaveformViewerEditorProvider(context)
    const providerRegistration = vscode.window.registerCustomEditorProvider(
      SurferWaveformViewerEditorProvider.viewType,
      provider,
      {
        webviewOptions: {
          retainContextWhenHidden: true,
        },
      }
    )

    return providerRegistration
  }
  private static readonly viewType = 'surfer.waveformViewer'

  constructor(private readonly context: vscode.ExtensionContext) {}
  async openCustomDocument(
    uri: vscode.Uri,
    _openContext: vscode.CustomDocumentOpenContext,
    _token: vscode.CancellationToken
  ): Promise<Wavefile> {
    return new Wavefile(uri)
  }
  async resolveCustomEditor(
    document: vscode.CustomDocument,
    webviewPanel: vscode.WebviewPanel,
    _token: vscode.CancellationToken
  ): Promise<void> {
    // Add the folder that the document lives in as a localResourceRoot
    const uriString = document.uri.toString()
    // TODO: this probably doesn't play well with windows
    const dirPath = uriString.substring(0, uriString.lastIndexOf('/'))

    webviewPanel.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.joinPath(this.context.extensionUri, 'surfer'),
        vscode.Uri.parse(dirPath, true),
      ],
    }

    webviewPanel.webview.html = await this.getHtmlForWebview(webviewPanel.webview)

    const documentUri = webviewPanel.webview.asWebviewUri(document.uri).toString()

    webviewPanel.webview.onDidReceiveMessage(async (message) => {
      switch (message.command) {
        case 'loaded': {
          console.log("Surfer got the loaded message from the web view")
          webviewPanel.webview.postMessage({command: "LoadUrl", url: documentUri})
          break;
        }

        case 'showOpenDialog': {
          const { kind, filters } = message
          // `filters` is an array of {name, extensions} objects from the Rust side.
          const filterRecord: Record<string, string[]> = {}
          for (const f of (filters ?? [])) {
            filterRecord[f.name] = f.extensions
          }
          const uris = await vscode.window.showOpenDialog({
            canSelectMany: false,
            filters: filterRecord,
            title: message.title ?? 'Open file',
          })
          if (uris && uris.length > 0) {
            // Grant the webview access to the directory containing the chosen file.
            const chosenDir = vscode.Uri.joinPath(uris[0], '..')
            webviewPanel.webview.options = {
              ...webviewPanel.webview.options,
              localResourceRoots: [
                ...(webviewPanel.webview.options.localResourceRoots ?? []),
                chosenDir,
              ],
            }
            const msg = openDialogMessage(kind, uris[0], webviewPanel.webview)
            if (msg !== null) {
              webviewPanel.webview.postMessage({ command: 'InjectMessage', message: msg })
            } else {
              console.log(`Surfer: unknown open-dialog kind '${kind}'`)
            }
          }
          break;
        }

        case 'showSaveDialog': {
          const { kind, filters } = message
          const filterRecord: Record<string, string[]> = {}
          for (const f of (filters ?? [])) {
            filterRecord[f.name] = f.extensions
          }
          // Derive a default file name extension from the first filter entry.
          const firstExt = filters?.[0]?.extensions?.[0]
          const defaultUri = firstExt
            ? vscode.Uri.file(path.join(require('os').homedir(), `surfer_state.${firstExt}`))
            : undefined
          const uri = await vscode.window.showSaveDialog({
            filters: filterRecord,
            title: message.title ?? 'Save file',
            defaultUri,
          })
          if (uri) {
            // Ask the webview for the encoded state, then write it via the
            // extension host which has full file-system access.
            webviewPanel.webview.postMessage({ command: 'RequestState', kind, targetUri: uri.toString() })
          }
          break;
        }

        case 'saveState': {
          // Sent by the webview in response to 'RequestState'. Contains the
          // encoded state string and the target URI chosen in showSaveDialog.
          const targetUri = vscode.Uri.parse(message.targetUri, true)
          const encoded = new TextEncoder().encode(message.state)
          try {
            await vscode.workspace.fs.writeFile(targetUri, encoded)
          } catch (e) {
            vscode.window.showErrorMessage(`Surfer: failed to save state file: ${e}`)
          }
          break;
        }

        default: {
          console.log(`Got unexpected command (${message.command}) from surfer web view`)
        }
      }
    })
  }

  // Get the static html used for the editor webviews.
  private async getHtmlForWebview(webview: vscode.Webview): Promise<string> {
    // Read index.html from disk
    const indexPath = vscode.Uri.joinPath(this.context.extensionUri, 'surfer', 'index.html')
    const contents = await vscode.workspace.fs.readFile(indexPath)

    // Replace local paths with paths derived from WebView URIs
    let html = new TextDecoder().decode(contents)

    const translate_url = (path: string[], in_html: string) => {
      const uriString = webview
        .asWebviewUri(vscode.Uri.joinPath(this.context.extensionUri, ...path))
        .toString()
      html = this.replaceAll(html, `["']${in_html}['"]`, `"${uriString}"`)
    }

    translate_url(["surfer", "manifest.json"], "/manifest.json");
    translate_url(["surfer", "surfer_bg.wasm"], "/surfer_bg.wasm");
    translate_url(["surfer", "surfer.js"], "/surfer.js");
    translate_url(["surfer", "sw.js"], "/sw.js");
    translate_url(["surfer", "integration.js"], "integration.js");

    // We need to wait until the new HTML has loaded before we can start injecting messages.
    // To detect this, we'll add a script at the every end of the html that sends a message
    // back to the host saying that the HTML is loaded.
    // A hacky but working way of doing this is to replace the </body> tag with a script
    // and a new </body> tag
    const vscodeApi = `acquireVsCodeApi()`
    const load_notifier = `
        (function() {
            const vscode = ${vscodeApi};

            // Expose native VS Code file-dialog helpers to the Rust/WASM side.
            // These are called by the \`vscode\` feature build via wasm_bindgen externs.
            window.vscode_show_open_dialog = function(kind, filtersJson) {
                const filters = JSON.parse(filtersJson);
                vscode.postMessage({ command: 'showOpenDialog', kind, filters });
            };
            window.vscode_show_save_dialog = function(kind, filtersJson) {
                const filters = JSON.parse(filtersJson);
                vscode.postMessage({ command: 'showSaveDialog', kind, filters });
            };

            // Listen for 'RequestState' from the host (triggered by showSaveDialog)
            // and reply with the current encoded state via get_state().
            window.addEventListener('message', async (event) => {
                if (event.data?.command === 'RequestState') {
                    const state = await get_state();
                    vscode.postMessage({ command: 'saveState', targetUri: event.data.targetUri, state });
                } else if (event.data?.command === 'InjectMessage') {
                    inject_message(event.data.message);
                }
            });

            vscode.postMessage({
                command: 'loaded',
            })
        }())`
    html = html.replaceAll("/*SURFER_SETUP_HOOKS*/", `${load_notifier}`)

    return html
  }

  private replaceAll(input: string, search: string, replacement: string): string {
    return input.replace(new RegExp(search, 'g'), replacement)
  }
}
