# postmarketOS no Moto G35 (manila / ums9620)

> Alvo de port **sem Android userspace**. B.A.S.E. só entrega assist (DT/earlycon/atlas).
> `generates_os: false` · flash **manual** · modem/GPU Unisoc = buraco longo.

Codinome stock: **manila** · SoC: **UMS9620** · Arch: **aarch64**

## 0. No B.A.S.E. (já feito)

```bash
./examples/pilot_moto_g35/run_path_a.sh
```

Usa: `out_real/handoff_external/`  
Ficheiros-chave: `dt/board-ums9620-wedge-merged.dtsi`, `cmdline/cmdline_earlycon.txt`, `atlas/`

## 1. Instalar pmbootstrap (máquina de build)

**Não uses `pipx install pmbootstrap`** — no PyPI está *yanked* / deprecated.

### Método oficial (git)

```bash
cd ~
git clone --depth=1 https://gitlab.postmarketos.org/postmarketOS/pmbootstrap.git
mkdir -p ~/.local/bin
ln -sf "$HOME/pmbootstrap/pmbootstrap.py" ~/.local/bin/pmbootstrap
# se "command not found":
grep -q '.local/bin' ~/.bashrc || echo 'PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
pmbootstrap --version
```

Docs: https://docs.postmarketos.org/pmbootstrap/main/installation.html

### Alternativa Debian/Ubuntu (pacote)

```bash
sudo apt update && sudo apt install -y pmbootstrap
pmbootstrap --version
```

(Em distros “fixed”, o pacote pode estar atrasado — preferir git se falhar.)

Depois:

```bash
pmbootstrap init
```

Respostas típicas (ajusta à doc actual):
- Work path: à tua escolha
- Device: **criar novo** → vendor `motorola` (ou `moto`), codename `manila`
- Arch: `aarch64`
- UI inicial: **none** / console (UI Phosh só depois de shell)
- Kernel: começar **downstream** (kernel Android/Lineage adaptado) — mainline ums9620 quase inexistente

## 2. Ligar o handoff ao device package

Depois do `init`, no `pmaports`:

```text
device/testing/device-motorola-manila/   # path exacto depende da categoria
device/testing/linux-motorola-manila/
```

### deviceinfo (mínimo útil)

```sh
deviceinfo_name="Motorola Moto G35 5G"
deviceinfo_manufacturer="Motorola"
deviceinfo_codename="motorola-manila"
deviceinfo_year="2024"
deviceinfo_arch="aarch64"
deviceinfo_chassis="handset"
deviceinfo_flash_method="fastboot"   # ou o que bootimg_analyze disser
# deviceinfo_dtb=...  quando tiveres DTB no kernel package
```

Analisa o `boot.img` stock (no piloto):

```bash
pmbootstrap bootimg_analyze examples/pilot_moto_g35/real_fw/boot-gki.img
# ou o boot.img que fores usar — preenche header/pagesize no deviceinfo
```

### Kernel + DT

1. Copia `handoff_external/dt/*.dtsi` para o tree do kernel (overlay / board).
2. Mete no cmdline do kernel / `deviceinfo` um earlycon de:

```text
earlycon=uart8250,mmio32,0x20200000,115200n8
```

3. UART0 clocks (já resolvidos no merged DTSI):

```dts
clocks = <&apapb_gate 7>, <&ap_clk 3>, <&ext_26m>;
clock-names = "enable", "uart", "source";
```

4. GICD `0x12000000` · GICR `0x12040000` · UFS `0x22000000` — ver `atlas/wedge_mmio_map.yaml`.

## 3. Build (externo — não no B.A.S.E.)

```bash
pmbootstrap checksum device-motorola-manila
pmbootstrap checksum linux-motorola-manila
pmbootstrap build linux-motorola-manila --arch=aarch64
pmbootstrap build device-motorola-manila
pmbootstrap install --no-fde    # gera rootfs/boot; sem claim de boot no silício
```

Exporta imagens:

```bash
pmbootstrap export
# → /tmp/postmarketOS-export/  (ou work path)
sha256sum ... > guarda para o receipt
```

## 4. Flash manual + watch

1. Unlock / fastboot conforme política do lab (≠ CI).
2. Flash **à mão** (`fastboot flash boot …` / partições que o deviceinfo indicar).
3. Volta ao B.A.S.E.:

```bash
./examples/pilot_moto_g35/out_real/handoff_external/lab/lab_watch_assist.sh
```

4. Preenche `lab/hw_boot_receipt.json` (`result`, `image_sha256`, `flashed`).

## 5. Ordem de vitórias (honesta)

| Meta | OK quando |
|------|-----------|
| P0 | `earlycon_seen` / dmesg na UART |
| P1 | initramfs / shell ash |
| P2 | rootfs monta (UFS — difícil) |
| P3 | display framebuffer |
| Depois | Wi‑Fi; **modem quase de certeza precisa blobs** — fora do “pure Linux” fácil |

## 6. O que não fazer

- Esperar mainline drop-in no ums9620
- Usar Halium/UT se o objectivo é “sem Android”
- Afirmar `generates_os` / flash automático via B.A.S.E.
- Flash no CI default

## Refs

- https://postmarketos.org  
- https://docs.postmarketos.org/pmbootstrap/  
- Handoff: `EXTERNAL_TREE.md` · `WEDGE_HANDOFF.md`  
- Docs oficiais port: wiki “Porting to a new device”
