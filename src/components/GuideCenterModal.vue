<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Icon } from '@iconify/vue';

const { t } = useI18n({
  useScope: 'local',
  messages: {
    en: {
      guide: {
        title: 'Guide Center',
        subtitle: 'Network setup, shortcuts & quick app launch guide',
        close: 'Close',
        nav: {
          browser: 'Open Web Browser',
          shortcut: 'Paste (.lnk) Shortcut',
          firewall: 'Firewall & Network Ports',
        },
        browser: {
          heading: 'Automatically Open Web Browser',
          description:
            'Set up a macro to automatically launch the Google Chrome web browser and navigate to a preset URL. Below are sample commands to use for the <strong>Shell Command</strong> field.',
          inUse: 'In Use',
          useTemplate: 'Use This Template',
        },
        shortcut: {
          heading: 'App Shortcuts & Copy as Path',
          description:
            'Detailed guide on how to drag and drop an app shortcut (.lnk) or get the exact file path on Windows so Companion can automatically parse it and launch it quickly.',
          step1Title: 'Find the app shortcut (.lnk) or original file (.exe)',
          step1Body: 'Open File Explorer and locate the shortcut on the Desktop or in the original install folder.',
          step2Title: 'Copy the Path (Copy as Path)',
          step2Body: 'Right-click the shortcut file and select <strong>"Copy as path"</strong> (or hold <code>Shift</code> + right-click and select "Copy as path" on Windows 10 and earlier).',
          step3Title: 'Switch to the App Configuration Tab',
          step3Body: 'Click the grid cell you want to assign in Companion, then create or switch to the <strong>"App"</strong> configuration tab.',
          step4Title: 'Paste the Path and Save',
          step4Body: 'Paste (Ctrl + V) directly into the path input field. The client automatically strips surrounding double quotes if present and handles the file-launch logic on the backend when activated.',
        },
        firewall: {
          heading: 'Fixing Firewall Errors & Port Conflicts',
          description: "When the Companion socket is blocked or there's a port conflict (Address already in use), your receiving device will show as offline.",
          step1Title: 'Allow Companion Through Windows Defender Firewall',
          step1Body: 'When you first launch Companion, click <strong>"Allow access"</strong> on the Windows notification prompt. If you accidentally dismissed it, go to <em>Control Panel -&gt; Windows Defender Firewall -&gt; Allow an app through firewall</em>, find <code>android-stream-desk</code>, and check the box for both <strong>Private</strong> and <strong>Public</strong>.',
          step2Title: 'Change the WebSocket Port Number (Port Conflict)',
          step2Body: "If another application is using the default port 8089, you'll need to switch to a different working port (e.g. 8090, 8092, etc.). Go to the <strong>Companion Connection Settings</strong> section below the network status in the Dashboard to change the port and save.",
          step3Title: 'Confirm Wi-Fi & LAN Are on the Same Subnet (AP Isolation)',
          step3Body: 'Make sure both the Companion computer and your Android Client device are connected to the same Router/Access Point on the LAN. Disable "AP Isolation / Guest Network" mode if it\'s enabled on your router.',
        },
      },
    },
    vi: {
      guide: {
        title: 'Trung tâm trợ giúp (Guide Center)',
        subtitle: 'Hướng dẫn thiết lập mạng, phím tắt & mở ứng dung nhanh',
        close: 'Đóng',
        nav: {
          browser: 'Mở Trình Duyệt Web',
          shortcut: 'Dán (.lnk) Shortcut',
          firewall: 'Tường Lửa & Cổng mạng',
        },
        browser: {
          heading: 'Tự động mở trình duyệt Web',
          description:
            'Thiết lập macro để tự động kích hoạt trình duyệt web Google Chrome và truy cập vào đường dẫn định sẵn. Dưới đây là các câu lệnh mẫu dùng cho mục gán <strong>Lệnh shell (Command)</strong>.',
          inUse: 'Đang sử dụng',
          useTemplate: 'Dùng mẫu này',
        },
        shortcut: {
          heading: 'Phím tắt ứng dụng & Copy as path',
          description:
            'Hướng dẫn chi tiết cách kéo thả phím tắt (.lnk) hoặc lấy đường dẫn tệp tin ứng dụng chính xác trên Windows để Companion tự động phân tích và kích hoạt nhanh.',
          step1Title: 'Tìm phím tắt ứng dụng (.lnk) hoặc file gốc (.exe)',
          step1Body: 'Mở File Explorer, tìm đến phím tắt ngoài Desktop hoặc trong thư mục cài đặt gốc.',
          step2Title: 'Sao chép đường dẫn (Copy as path)',
          step2Body: 'Chuột phải vào tệp phím tắt, chọn <strong>"Copy as path"</strong> (hoặc giữ phím <code>Shift</code> + chuột phải và chọn "Copy as path" trên phiên bản Windows 10 trở xuống).',
          step3Title: 'Chuyển sang tab cấu hình App',
          step3Body: 'Click chọn ô lưới muốn gán ở Companion, tạo/chuyển qua tab cấu hình loại <strong>"App"</strong>.',
          step4Title: 'Dán đường dẫn và lưu trữ',
          step4Body: 'Dán (Ctrl + V) trực tiếp vào ô nhập đường dẫn. Client tự động loại bỏ dấu ngoặc kép kép nếu có và thực hiện mở rộng tệp logic dưới backend khi kích hoạt.',
        },
        firewall: {
          heading: 'Khắc phục lỗi Tường lửa & Trùng cổng mạng',
          description: 'Khi socket Companion bị chặn hoặc đụng độ cổng mạng (Address already in use), thiết bị nhận tin của bạn sẽ hiển thị ngoại tuyến.',
          step1Title: 'Cho phép Companion qua Windows Defender Firewall',
          step1Body: 'Khi khởi chạy Companion lần đầu, hãy click <strong>"Allow access"</strong> trên bảng thông báo Windows. Nếu đã lỡ bỏ qua, hãy vào <em>Control Panel -&gt; Windows Defender Firewall -&gt; Allow an app through firewall</em>, tìm <code>android-stream-desk</code> và bật tick chọn cho cả <strong>Private</strong> và <strong>Public</strong>.',
          step2Title: 'Thay đổi số cổng WebSocket (Port Conflict)',
          step2Body: 'Nếu có ứng dụng khác đang chiếm dụng cổng mặc định 8089, bạn cần đổi sang một cổng khác hoạt động (ví dụ: 8090, 8092, v.v.). Đi tới phần <strong>Cài đặt kết nối hệ thống Companion</strong> bên dưới mục trạng thái mạng của Dashboard để sửa lại cổng và lưu.',
          step3Title: 'Xác nhận Wi-Fi & LAN cùng subnet (AP isolation)',
          step3Body: 'Đảm bảo cả máy tính Companion lẫn thiết bị Android Client của bạn kết nối vào cùng 1 Router/Access Point mạng LAN. Hãy tắt chế độ "AP Isolation / Guest Network" nếu được bật trên router của bạn.',
        },
      },
    },
  },
});

const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    activeTopic?: 'browser' | 'shortcut' | 'firewall';
  }>(),
  {
    activeTopic: 'browser',
  }
);

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void;
  (e: 'apply-template', command: string): void;
}>();

const activeTab = ref<'browser' | 'shortcut' | 'firewall'>('browser');

watch(
  () => props.modelValue,
  (isOpen) => {
    if (isOpen) {
      activeTab.value = props.activeTopic || 'browser';
    }
  }
);

watch(
  () => props.activeTopic,
  (newTopic) => {
    if (newTopic) {
      activeTab.value = newTopic;
    }
  }
);

const isMac = computed(() => {
  return (
    navigator.userAgent.toLowerCase().includes('mac') ||
    navigator.platform.toLowerCase().includes('mac')
  );
});

const winCommand = 'start "" chrome "https://facebook.com"';
const macCommand = 'open -a "Google Chrome" "https://facebook.com"';

function close() {
  emit('update:modelValue', false);
}

function applyTemplate(cmd: string) {
  emit('apply-template', cmd);
  close();
}
</script>

<template>
  <transition name="fade">
    <div
      v-if="modelValue"
      class="fixed inset-0 z-[60] flex items-center justify-center bg-black/85 backdrop-blur-md p-4 animate-fade-in"
      @click.self="close"
    >
      <div class="guide-modal w-[640px] max-w-full max-h-[80vh] flex flex-col p-5 gap-4 relative">
        <!-- Header -->
        <div class="flex items-center justify-between pb-3 border-b border-cyan-500/15">
          <div class="flex items-center gap-2.5">
            <div class="h-8 w-8 rounded-lg bg-gradient-to-br from-cyan-500 to-fuchsia-500 shadow-[0_0_16px_rgba(6,182,212,0.2)] flex items-center justify-center">
              <Icon icon="lucide:help-circle" class="text-base text-white" />
            </div>
            <div>
              <h2 class="text-xs font-bold text-slate-50 uppercase tracking-wider">
                {{ t('guide.title') }}
              </h2>
              <p class="text-[8px] text-slate-500 mt-0.5">{{ t('guide.subtitle') }}</p>
            </div>
          </div>
          <button
            type="button"
            class="text-slate-400 hover:text-cyan-400 transition-colors cursor-pointer"
            :title="t('guide.close')"
            @click="close"
          >
            <Icon icon="lucide:x" class="text-lg" />
          </button>
        </div>

        <!-- Body Layout -->
        <div class="flex flex-1 min-h-0 gap-4 overflow-hidden">
          <!-- Left Navigation Menu -->
          <nav class="w-1/3 flex flex-col gap-1 border-r border-cyan-500/10 pr-3">
            <button
              type="button"
              class="menu-btn flex items-center gap-2 px-3 py-2.5 rounded-lg text-left text-[10px] font-bold uppercase transition-all"
              :class="activeTab === 'browser' ? 'menu-btn--active' : ''"
              @click="activeTab = 'browser'"
            >
              <Icon icon="lucide:chrome" class="text-xs text-cyan-400 shrink-0" />
              <span>{{ t('guide.nav.browser') }}</span>
            </button>
            <button
              type="button"
              class="menu-btn flex items-center gap-2 px-3 py-2.5 rounded-lg text-left text-[10px] font-bold uppercase transition-all"
              :class="activeTab === 'shortcut' ? 'menu-btn--active' : ''"
              @click="activeTab = 'shortcut'"
            >
              <Icon icon="lucide:external-link" class="text-xs text-cyan-400 shrink-0" />
              <span>{{ t('guide.nav.shortcut') }}</span>
            </button>
            <button
              type="button"
              class="menu-btn flex items-center gap-2 px-3 py-2.5 rounded-lg text-left text-[10px] font-bold uppercase transition-all"
              :class="activeTab === 'firewall' ? 'menu-btn--active' : ''"
              @click="activeTab = 'firewall'"
            >
              <Icon icon="lucide:shield-alert" class="text-xs text-rose-450 shrink-0 animate-pulse" />
              <span>{{ t('guide.nav.firewall') }}</span>
            </button>
          </nav>

          <!-- Right Content Container -->
          <div class="flex-1 overflow-y-auto pl-1 pr-1 flex flex-col gap-4 text-slate-350">
            <!-- Tab: Browser commands -->
            <div v-if="activeTab === 'browser'" class="flex flex-col gap-4">
              <div>
                <h3 class="text-[11px] font-bold text-slate-205 uppercase tracking-wide mb-1 text-cyan-400">
                  {{ t('guide.browser.heading') }}
                </h3>
                <p class="text-[9px] text-slate-400 leading-relaxed" v-html="t('guide.browser.description')"></p>
              </div>

              <!-- MacOS Segment -->
              <div
                class="os-box p-3 rounded-lg border flex flex-col gap-2"
                :class="isMac ? 'bg-cyan-500/5 border-cyan-400/30' : 'bg-slate-900/40 border-slate-800'"
              >
                <div class="flex items-center justify-between">
                  <span class="text-[9px] font-bold uppercase flex items-center gap-1" :class="isMac ? 'text-cyan-400' : 'text-slate-400'">
                    <Icon icon="lucide:apple" /> macOS
                    <span v-if="isMac" class="ml-1 px-1.5 py-0.5 text-[8px] bg-cyan-500/20 text-cyan-400 rounded-full font-semibold normal-case">{{ t('guide.browser.inUse') }}</span>
                  </span>
                  <button
                    type="button"
                    class="use-template-btn text-[9px] font-bold px-2 py-1 rounded border border-cyan-400 cursor-pointer"
                    @click="applyTemplate(macCommand)"
                  >
                    {{ t('guide.browser.useTemplate') }}
                  </button>
                </div>
                <code class="text-[10px] font-mono bg-black/40 p-2 rounded block whitespace-pre-wrap select-all select-text border border-black/40">
                  {{ macCommand }}
                </code>
              </div>

              <!-- Windows Segment -->
              <div
                class="os-box p-3 rounded-lg border flex flex-col gap-2"
                :class="!isMac ? 'bg-cyan-500/5 border-cyan-400/30' : 'bg-slate-900/40 border-slate-800'"
              >
                <div class="flex items-center justify-between">
                  <span class="text-[9px] font-bold uppercase flex items-center gap-1" :class="!isMac ? 'text-cyan-400' : 'text-slate-400'">
                    <Icon icon="lucide:monitor" /> Windows
                    <span v-if="!isMac" class="ml-1 px-1.5 py-0.5 text-[8px] bg-cyan-500/20 text-cyan-400 rounded-full font-semibold normal-case">{{ t('guide.browser.inUse') }}</span>
                  </span>
                  <button
                    type="button"
                    class="use-template-btn text-[9px] font-bold px-2 py-1 rounded border border-cyan-400 cursor-pointer"
                    @click="applyTemplate(winCommand)"
                  >
                    {{ t('guide.browser.useTemplate') }}
                  </button>
                </div>
                <code class="text-[10px] font-mono bg-black/40 p-2 rounded block whitespace-pre-wrap select-all select-text border border-black/40">
                  {{ winCommand }}
                </code>
              </div>
            </div>

            <!-- Tab: Shortcut link copy-as-path instructions -->
            <div v-else-if="activeTab === 'shortcut'" class="flex flex-col gap-4 text-[9px] leading-relaxed">
              <div>
                <h3 class="text-[11px] font-bold text-slate-205 uppercase tracking-wide mb-1 text-cyan-400">
                  {{ t('guide.shortcut.heading') }}
                </h3>
                <p class="text-slate-400">
                  {{ t('guide.shortcut.description') }}
                </p>
              </div>

              <!-- Step Guide List -->
              <div class="flex flex-col gap-2.5">
                <div class="step-card flex gap-3 p-2.5 rounded bg-slate-900/30 border border-slate-800">
                  <span class="step-num text-xs font-bold text-cyan-400 w-5 h-5 rounded-full bg-cyan-500/10 flex items-center justify-center shrink-0">1</span>
                  <div>
                    <span class="font-bold text-slate-200 block text-[10px] mb-0.5">{{ t('guide.shortcut.step1Title') }}</span>
                    <span class="text-slate-400">{{ t('guide.shortcut.step1Body') }}</span>
                  </div>
                </div>

                <div class="step-card flex gap-3 p-2.5 rounded bg-slate-900/30 border border-slate-800">
                  <span class="step-num text-xs font-bold text-cyan-400 w-5 h-5 rounded-full bg-cyan-500/10 flex items-center justify-center shrink-0">2</span>
                  <div>
                    <span class="font-bold text-slate-200 block text-[10px] mb-0.5">{{ t('guide.shortcut.step2Title') }}</span>
                    <span class="text-slate-400" v-html="t('guide.shortcut.step2Body')"></span>
                  </div>
                </div>

                <div class="step-card flex gap-3 p-2.5 rounded bg-slate-900/30 border border-slate-800">
                  <span class="step-num text-xs font-bold text-cyan-400 w-5 h-5 rounded-full bg-cyan-500/10 flex items-center justify-center shrink-0">3</span>
                  <div>
                    <span class="font-bold text-slate-200 block text-[10px] mb-0.5">{{ t('guide.shortcut.step3Title') }}</span>
                    <span class="text-slate-400" v-html="t('guide.shortcut.step3Body')"></span>
                  </div>
                </div>

                <div class="step-card flex gap-3 p-2.5 rounded bg-slate-900/30 border border-slate-800">
                  <span class="step-num text-xs font-bold text-cyan-400 w-5 h-5 rounded-full bg-cyan-500/10 flex items-center justify-center shrink-0">4</span>
                  <div>
                    <span class="font-bold text-slate-200 block text-[10px] mb-0.5">{{ t('guide.shortcut.step4Title') }}</span>
                    <span class="text-slate-400">{{ t('guide.shortcut.step4Body') }}</span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Tab: LAN / Firewall Troubleshooting -->
            <div v-else-if="activeTab === 'firewall'" class="flex flex-col gap-4 text-[9px] leading-relaxed">
              <div>
                <h3 class="text-[11px] font-bold text-rose-400 uppercase tracking-wide mb-1 flex items-center gap-1.5">
                  <Icon icon="lucide:shield-alert" class="text-xs shrink-0" />
                  {{ t('guide.firewall.heading') }}
                </h3>
                <p class="text-slate-400">
                  {{ t('guide.firewall.description') }}
                </p>
              </div>

              <!-- Step Guide List -->
              <div class="flex flex-col gap-2.5">
                <div class="step-card flex gap-3 p-2.5 rounded bg-slate-900/30 border border-slate-800">
                  <span class="step-num text-xs font-bold text-rose-450 w-5 h-5 rounded-full bg-rose-500/10 flex items-center justify-center shrink-0">1</span>
                  <div>
                    <span class="font-bold text-slate-200 block text-[10px] mb-0.5">{{ t('guide.firewall.step1Title') }}</span>
                    <span class="text-slate-400" v-html="t('guide.firewall.step1Body')"></span>
                  </div>
                </div>

                <div class="step-card flex gap-3 p-2.5 rounded bg-slate-900/30 border border-slate-800">
                  <span class="step-num text-xs font-bold text-rose-400 w-5 h-5 rounded-full bg-rose-500/10 flex items-center justify-center shrink-0">2</span>
                  <div>
                    <span class="font-bold text-slate-200 block text-[10px] mb-0.5">{{ t('guide.firewall.step2Title') }}</span>
                    <span class="text-slate-400" v-html="t('guide.firewall.step2Body')"></span>
                  </div>
                </div>

                <div class="step-card flex gap-3 p-2.5 rounded bg-slate-900/30 border border-slate-800">
                  <span class="step-num text-xs font-bold text-rose-400 w-5 h-5 rounded-full bg-rose-500/10 flex items-center justify-center shrink-0">3</span>
                  <div>
                    <span class="font-bold text-slate-200 block text-[10px] mb-0.5">{{ t('guide.firewall.step3Title') }}</span>
                    <span class="text-slate-400">{{ t('guide.firewall.step3Body') }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.guide-modal {
  background: rgba(4, 10, 24, 0.95);
  border: 1px solid rgba(0, 240, 255, 0.12);
  box-shadow:
    0 0 0 1px rgba(0, 240, 255, 0.03),
    0 16px 48px -16px rgba(0, 0, 0, 0.6),
    inset 0 1px 0 rgba(255, 255, 255, 0.015);
  border-radius: 16px;
  backdrop-filter: blur(16px);
}

.menu-btn {
  background: transparent;
  color: rgba(255, 255, 255, 0.5);
  border: 1px solid transparent;
}

.menu-btn:hover {
  background: rgba(0, 240, 255, 0.04);
  color: rgba(255, 255, 255, 0.9);
}

.menu-btn--active,
.menu-btn--active:hover {
  background: rgba(0, 240, 255, 0.08);
  border-color: rgba(0, 240, 255, 0.14);
  color: #fff;
  box-shadow: 0 0 12px rgba(0, 240, 255, 0.04);
}

.use-template-btn {
  background: rgba(6, 182, 212, 0.08);
  color: rgb(34, 211, 238);
  border-color: rgba(6, 182, 212, 0.3);
  transition: all 0.2s;
}

.use-template-btn:hover {
  background: rgba(6, 182, 212, 0.2);
  border-color: rgb(34, 211, 238);
  box-shadow: 0 0 10px rgba(6, 182, 212, 0.25);
}

.step-card {
  transition: border-color 0.2s;
}
.step-card:hover {
  border-color: rgba(0, 240, 255, 0.15);
}
</style>