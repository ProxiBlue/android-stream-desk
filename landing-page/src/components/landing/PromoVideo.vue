<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue';
import { Pause, Play, Volume2, VolumeX } from 'lucide-vue-next';

defineProps<{
  accentColor: string;
}>();

const videoRef = useTemplateRef<HTMLVideoElement>('videoRef');
const muted = ref(true);
const playing = ref(false);
// người dùng tự bấm dừng thì không tự phát lại khi cuộn quay về
let pausedByUser = false;
let observer: IntersectionObserver | null = null;

function syncPlaying() {
  playing.value = !!videoRef.value && !videoRef.value.paused;
}

function toggleSound() {
  const v = videoRef.value;
  if (!v) return;
  if (v.muted) {
    // lần đầu bật tiếng: phát lại từ đầu để nghe đủ lời đọc
    if (muted.value) v.currentTime = 0;
    v.muted = false;
    pausedByUser = false;
    void v.play().catch(() => {});
  } else {
    v.muted = true;
  }
  muted.value = v.muted;
}

function togglePlay() {
  const v = videoRef.value;
  if (!v) return;
  if (v.paused) {
    pausedByUser = false;
    void v.play().catch(() => {});
  } else {
    pausedByUser = true;
    v.pause();
  }
}

onMounted(() => {
  const v = videoRef.value;
  if (!v) return;
  const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (reduceMotion) return; // không tự phát; người xem bấm Play nếu muốn

  // chỉ phát khi video nằm trong khung nhìn, rời khỏi thì dừng để tiết kiệm CPU/băng thông
  observer = new IntersectionObserver(
    ([entry]) => {
      if (entry.isIntersecting && !pausedByUser) void v.play().catch(() => {});
      else if (!entry.isIntersecting && !v.paused) v.pause();
    },
    { threshold: 0.35 },
  );
  observer.observe(v);
});

onBeforeUnmount(() => observer?.disconnect());
</script>

<template>
  <figure
    class="group relative overflow-hidden rounded-2xl border border-white/[0.08] bg-[#000212] shadow-[0_32px_120px_rgba(0,0,0,0.42)]"
  >
    <video
      ref="videoRef"
      class="block aspect-video w-full bg-black object-cover"
      src="/media/promo.mp4"
      poster="/media/promo-poster.jpg"
      width="1920"
      height="1080"
      muted
      loop
      playsinline
      preload="metadata"
      aria-label="Video giới thiệu Android Stream Desk"
      @play="syncPlaying"
      @pause="syncPlaying"
      @volumechange="muted = videoRef?.muted ?? true"
      @click="togglePlay"
    ></video>

    <div class="pointer-events-none absolute inset-x-0 bottom-0 h-24 bg-[linear-gradient(180deg,rgba(0,2,18,0)_0%,rgba(0,2,18,0.55)_100%)]"></div>

    <div class="absolute bottom-3 right-3 flex items-center gap-2 sm:bottom-4 sm:right-4">
      <button
        type="button"
        class="flex h-9 w-9 items-center justify-center rounded-lg border border-white/[0.12] bg-black/50 text-white/80 backdrop-blur-sm transition-colors hover:bg-black/70 hover:text-white focus:outline-none focus-visible:ring-2"
        :style="{ '--tw-ring-color': accentColor }"
        :aria-label="playing ? 'Tạm dừng video' : 'Phát video'"
        @click="togglePlay"
      >
        <Pause v-if="playing" class="h-4 w-4" />
        <Play v-else class="h-4 w-4" />
      </button>
      <button
        type="button"
        class="flex h-9 items-center gap-1.5 rounded-lg border border-white/[0.12] bg-black/50 px-3 text-[11px] font-medium text-white/80 backdrop-blur-sm transition-colors hover:bg-black/70 hover:text-white focus:outline-none focus-visible:ring-2"
        :style="{ '--tw-ring-color': accentColor }"
        :aria-pressed="!muted"
        @click="toggleSound"
      >
        <VolumeX v-if="muted" class="h-4 w-4" />
        <Volume2 v-else class="h-4 w-4" :style="{ color: accentColor }" />
        <span>{{ muted ? 'Bật tiếng' : 'Tắt tiếng' }}</span>
      </button>
    </div>
  </figure>
</template>
