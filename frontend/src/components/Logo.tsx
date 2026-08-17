import React from "react";
import Image from "next/image";
import { Dialog, DialogContent, DialogTitle, DialogTrigger } from "./ui/dialog";
import { VisuallyHidden } from "./ui/visually-hidden";
import { About } from "./About";
import { useTranslation } from "react-i18next";

interface LogoProps {
    isCollapsed: boolean;
}

const Logo = React.forwardRef<HTMLButtonElement, LogoProps>(({ isCollapsed }, ref) => {
  const { t } = useTranslation('common');

  return (
    <Dialog aria-describedby={undefined}>
      {isCollapsed ? (
        <DialogTrigger asChild>
          <button ref={ref} className="mb-2 flex cursor-pointer items-center justify-start border-none bg-transparent p-0 transition-opacity hover:opacity-80" aria-label={t('aboutMingtily')}>
            <Image src="/logo-collapsed.png" alt={t('appLogo')} width={32} height={32} />
          </button>
        </DialogTrigger>
      ) : (
        <DialogTrigger asChild>
          <button ref={ref} className="mb-2 flex w-full cursor-pointer items-center justify-center gap-2 rounded-full border border-sky-100 bg-sky-50 px-3 py-1 text-lg font-semibold text-slate-700 transition-colors hover:bg-sky-100">
            <Image src="/logo-collapsed.png" alt="" width={28} height={28} aria-hidden="true" />
            <span>Mingtily</span>
          </button>
        </DialogTrigger>
      )}
      <DialogContent>
        <VisuallyHidden>
          <DialogTitle>{t('aboutMingtily')}</DialogTitle>
        </VisuallyHidden>
        <About />
      </DialogContent>
    </Dialog>
  );
});

Logo.displayName = "Logo";

export default Logo;
