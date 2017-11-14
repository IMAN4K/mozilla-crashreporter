/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*- */
/* vim: set ts=8 sts=2 et sw=2 tw=80: */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#include "nsExceptionHandler.h"
#include "nsExceptionHandlerUtils.h"

namespace CrashReporter {

void
AnnotateOOMAllocationSize(size_t size)
{
}

void
AnnotateTexturesSize(size_t size)
{
}

void
AnnotatePendingIPC(size_t aNumOfPendingIPC,
                   uint32_t aTopPendingIPCCount,
                   const char* aTopPendingIPCName,
                   uint32_t aTopPendingIPCType)
{
}

nsresult
SetExceptionHandler(nsIFile* aXREDirectory,
                    bool force/*=false*/)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

bool
GetEnabled()
{
  return false;
}

bool
GetMinidumpPath(nsAString& aPath)
{
  return false;
}

nsresult
SetMinidumpPath(const nsAString& aPath)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

nsresult
SetupExtraData(nsIFile* aAppDataDirectory,
               const nsACString& aBuildID)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

nsresult
UnsetExceptionHandler()
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

nsresult
AnnotateCrashReport(const nsACString& key,
                    const nsACString& data)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

nsresult
RemoveCrashReportAnnotation(const nsACString& key)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

nsresult
SetGarbageCollecting(bool collecting)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

void
SetEventloopNestingLevel(uint32_t level)
{
}

void
SetMinidumpAnalysisAllThreads()
{
}

nsresult
AppendAppNotesToCrashReport(const nsACString& data)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

bool
GetAnnotation(const nsACString& key, nsACString& data)
{
  return false;
}

nsresult
RegisterAppMemory(void* ptr, size_t length)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

nsresult
UnregisterAppMemory(void* ptr)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

void
SetIncludeContextHeap(bool aValue)
{
}

bool
GetServerURL(nsACString& aServerURL)
{
  return false;
}

nsresult
SetServerURL(const nsACString& aServerURL)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

nsresult
SetRestartArgs(int argc, char** argv)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

#ifdef XP_WIN32
nsresult
WriteMinidumpForException(EXCEPTION_POINTERS* aExceptionInfo)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}
#endif

#ifdef XP_LINUX
bool
WriteMinidumpForSigInfo(int signo,
                        siginfo_t* info,
                        void* uc)
{
  return false;
}
#endif

#ifdef XP_MACOSX
nsresult
AppendObjCExceptionInfoToAppNotes(void *inException)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}
#endif

nsresult
GetSubmitReports(bool* aSubmitReports)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

nsresult
SetSubmitReports(bool aSubmitReports)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

void
SetProfileDirectory(nsIFile* aDir)
{
}

void
SetUserAppDataDirectory(nsIFile* aDir)
{
}

void
UpdateCrashEventsDir()
{
}

bool
GetCrashEventsDir(nsAString& aPath)
{
  return false;
}

void
SetMemoryReportFile(nsIFile* aFile)
{
}

nsresult
GetDefaultMemoryReportFile(nsIFile** aFile)
{
  return NS_ERROR_NOT_IMPLEMENTED;
}

void
SetTelemetrySessionId(const nsACString& id)
{
}

void
DeleteMinidumpFilesForID(const nsAString& id)
{
}

bool
GetMinidumpForID(const nsAString& id, nsIFile** minidump)
{
  return false;
}

bool
GetIDFromMinidump(nsIFile* minidump, nsAString& id)
{
  return false;
}

bool
GetExtraFileForID(const nsAString& id,
                  nsIFile** extraFile)
{
  return false;
}

bool
GetExtraFileForMinidump(nsIFile* minidump,
                        nsIFile** extraFile)
{
  return false;
}

bool
AppendExtraData(const nsAString& id,
                const AnnotationTable& data)
{
  return false;
}

bool
AppendExtraData(nsIFile* extraFile,
                const AnnotationTable& data)
{
  return false;
}

void
OOPInit()
{
}

void
GetChildProcessTmpDir(nsIFile** aOutTmpDir)
{
}

#if defined(XP_WIN) || defined(XP_MACOSX)
const char*
GetChildNotificationPipe()
{
  return nullptr;
}
#endif

#ifdef MOZ_CRASHREPORTER_INJECTOR
void
InjectCrashReporterIntoProcess(DWORD processID, InjectorCrashCallback* cb)
{
}

void
UnregisterInjectorCallback(DWORD processID)
{
}

#endif // MOZ_CRASHREPORTER_INJECTOR

bool
GetLastRunCrashID(nsAString& id)
{
  return false;
}

#if defined(XP_WIN) || defined(XP_MACOSX)
void
InitChildProcessTmpDir(nsIFile* aDirOverride)
{
}
#endif // defined(XP_WIN) || defined(XP_MACOSX)

#if defined(XP_WIN)
bool
SetRemoteExceptionHandler(const nsACString& crashPipe)
{
  return false;
}

#elif defined(XP_LINUX) || defined(OS_BSD) || defined(OS_SOLARIS)

bool
CreateNotificationPipeForChild(int* childCrashFd, int* childCrashRemapFd)
{
  return false;
}

bool
SetRemoteExceptionHandler()
{
  return false;
}

#elif defined(XP_MACOSX)

bool
SetRemoteExceptionHandler(const nsACString& crashPipe)
{
  return false;
}
#endif  // XP_WIN

bool
TakeMinidumpForChild(uint32_t childPid, nsIFile** dump, uint32_t* aSequence)
{
  return false;
}

void
RenameAdditionalHangMinidump(nsIFile* minidump, nsIFile* childMinidump,
                             const nsACString& name)
{
}

ThreadId
CurrentThreadId()
{
  return -1;
}

bool
TakeMinidump(nsIFile** aResult, bool aMoveToPending)
{
  return false;
}

void
CreateMinidumpsAndPair(ProcessHandle aTargetPid,
                       ThreadId aTargetBlamedThread,
                       const nsACString& aIncomingPairName,
                       nsIFile* aIncomingDumpToPair,
                       nsIFile** aMainDumpOut,
                       std::function<void(bool)>&& aCallback,
                       bool aAsync)
{
}

bool
CreateAdditionalChildMinidump(ProcessHandle childPid,
                              ThreadId childBlamedThread,
                              nsIFile* parentMinidump,
                              const nsACString& name)
{
  return false;
}

bool
UnsetRemoteExceptionHandler()
{
  return false;
}

#if defined(MOZ_WIDGET_ANDROID)
void
SetNotificationPipeForChild(int childCrashFd)
{
}

void
AddLibraryMapping(const char* library_name,
                  uintptr_t   start_address,
                  size_t      mapping_length,
                  size_t      file_offset)
{
}
#endif

// From ThreadAnnotation.cpp

void
InitThreadAnnotation()
{
}

void
SetCurrentThreadName(const char* aName)
{
}

void
GetFlatThreadAnnotation(const std::function<void(const char*)>& aCallback)
{
}

void
ShutdownThreadAnnotation()
{
}

} // namespace CrashReporter
